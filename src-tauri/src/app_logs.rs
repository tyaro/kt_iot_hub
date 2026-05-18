use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};

use tracing_subscriber::fmt::MakeWriter;

const APP_LOG_MAX_LINES: usize = 5000;

fn log_buffer() -> &'static Mutex<VecDeque<String>> {
    static LOGS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
    LOGS.get_or_init(|| Mutex::new(VecDeque::with_capacity(APP_LOG_MAX_LINES)))
}

fn push_line(line: String) {
    if line.trim().is_empty() {
        return;
    }

    if let Ok(mut logs) = log_buffer().lock() {
        if logs.len() >= APP_LOG_MAX_LINES {
            logs.pop_front();
        }
        logs.push_back(line);
    }
}

pub fn list_logs(limit: usize) -> Vec<String> {
    let limit = limit.clamp(1, APP_LOG_MAX_LINES);
    if let Ok(logs) = log_buffer().lock() {
        let total = logs.len();
        let start = total.saturating_sub(limit);
        return logs.iter().skip(start).cloned().collect();
    }
    Vec::new()
}

pub fn clear_logs() {
    if let Ok(mut logs) = log_buffer().lock() {
        logs.clear();
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AppLogMakeWriter;

impl<'a> MakeWriter<'a> for AppLogMakeWriter {
    type Writer = AppLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        AppLogWriter {
            stderr: io::stderr(),
            line_buffer: Vec::new(),
        }
    }
}

pub struct AppLogWriter {
    stderr: io::Stderr,
    line_buffer: Vec<u8>,
}

impl AppLogWriter {
    fn flush_lines(&mut self) {
        while let Some(position) = self.line_buffer.iter().position(|byte| *byte == b'\n') {
            let mut line = self.line_buffer.drain(..=position).collect::<Vec<_>>();
            if line.last().copied() == Some(b'\n') {
                line.pop();
            }
            if line.last().copied() == Some(b'\r') {
                line.pop();
            }

            let text = String::from_utf8_lossy(&line).to_string();
            push_line(text);
        }
    }
}

impl Write for AppLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stderr.write_all(buf)?;
        self.line_buffer.extend_from_slice(buf);
        self.flush_lines();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stderr.flush()
    }
}

impl Drop for AppLogWriter {
    fn drop(&mut self) {
        if !self.line_buffer.is_empty() {
            let text = String::from_utf8_lossy(&self.line_buffer).to_string();
            push_line(text);
            self.line_buffer.clear();
        }
    }
}
