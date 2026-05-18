use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use anyhow::{anyhow, Context, Result};

use crate::joywatcher_artifacts::JoyWatcherArtifacts;
use crate::path_utils::{bridge_exe_candidates, BRIDGE_EXE_NAME};

pub struct JoyWatcherBridgeProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    connected: bool,
    connection: Option<BridgeConnectionSettings>,
    mode: BridgeMode,
    exe_path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BridgeConnectionSettings {
    pub endpoint: Option<String>,
    pub user_id: i32,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BridgeReadValue {
    pub native_tag_id: i32,
    pub quality: String,
    pub value: BridgeValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeValue {
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMode {
    Mock,
    Dll,
}

impl BridgeMode {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Dll => "dll",
        }
    }
}

impl JoyWatcherBridgeProcess {
    pub fn start(artifacts: &JoyWatcherArtifacts) -> Result<Self> {
        let exe_path = resolve_bridge_exe_path()?.ok_or_else(|| {
            anyhow!(
                "JoyWatcher bridge executable not found. Looked for '{}' in known bridge locations.",
                BRIDGE_EXE_NAME
            )
        })?;

        let mode = if artifacts.dll_path.is_some() {
            BridgeMode::Dll
        } else {
            BridgeMode::Mock
        };

        let mut command = Command::new(&exe_path);
        command
            .arg("--mode")
            .arg(mode.as_arg())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        if let (BridgeMode::Dll, Some(dll_path)) = (mode, artifacts.dll_path.as_ref()) {
            command.arg("--dll-path").arg(dll_path);
        }

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn JoyWatcher bridge: {}", exe_path.display()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to capture bridge stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture bridge stdout"))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            connected: false,
            connection: None,
            mode,
            exe_path,
        })
    }

    pub fn exe_path(&self) -> &Path {
        &self.exe_path
    }

    pub fn mode(&self) -> BridgeMode {
        self.mode
    }

    pub fn ping(&mut self) -> Result<String> {
        let response = self.send_request(r#"{"type":"ping"}"#)?;
        if !response.contains(r#""type":"pong""#) {
            return Err(anyhow!("unexpected bridge ping response: {}", response));
        }
        Ok(response)
    }

    pub fn ensure_connection(&mut self, settings: &BridgeConnectionSettings) -> Result<()> {
        if self.connected && self.connection.as_ref() == Some(settings) {
            return Ok(());
        }

        if self.connected {
            self.disconnect()?;
        }

        self.connect(settings)?;
        Ok(())
    }

    pub fn connect(&mut self, settings: &BridgeConnectionSettings) -> Result<String> {
        let mut request = String::from("{\"type\":\"connect\"");
        if let Some(endpoint) = settings.endpoint.as_deref() {
            request.push_str(&format!(",\"endpoint\":\"{}\"", escape_json_string(endpoint)));
        }
        request.push_str(&format!(",\"user_id\":{}", settings.user_id));
        request.push_str(&format!(",\"password\":\"{}\"}}", escape_json_string(&settings.password)));

        let response = self.send_request(&request)?;
        if !response.contains(r#""type":"connected""#) {
            return Err(anyhow!("unexpected bridge connect response: {}", response));
        }
        self.connected = true;
        self.connection = Some(settings.clone());
        Ok(response)
    }

    pub fn disconnect(&mut self) -> Result<String> {
        let response = self.send_request(r#"{"type":"disconnect"}"#)?;
        if !response.contains(r#""type":"disconnected""#) {
            return Err(anyhow!("unexpected bridge disconnect response: {}", response));
        }
        self.connected = false;
        self.connection = None;
        Ok(response)
    }

    pub fn read_tags(&mut self, tag_ids: &[i32]) -> Result<Vec<BridgeReadValue>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }

        let request = format!(
            r#"{{"type":"read","request_id":"joywatcher-read","tag_ids":[{}]}}"#,
            tag_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        let response = self.send_request(&request)?;
        parse_read_result(&response)
    }

    fn send_request(&mut self, request: &str) -> Result<String> {
        writeln!(self.stdin, "{}", request).context("failed to write bridge request")?;
        self.stdin.flush().context("failed to flush bridge stdin")?;

        loop {
            let mut line = String::new();
            let bytes_read = self
                .stdout
                .read_line(&mut line)
                .context("failed to read bridge response")?;

            if bytes_read == 0 {
                return Err(anyhow!("bridge returned EOF before responding"));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if !trimmed.starts_with('{') {
                continue;
            }

            return Ok(trimmed.to_string());
        }
    }
}

impl Drop for JoyWatcherBridgeProcess {
    fn drop(&mut self) {
        if self.connected {
            let _ = self.disconnect();
        }

        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn resolve_bridge_exe_path() -> Result<Option<PathBuf>> {
    for candidate in bridge_exe_candidates() {
        if candidate.exists() {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

fn parse_read_result(response: &str) -> Result<Vec<BridgeReadValue>> {
    if !response.contains(r#""type":"readResult""#) {
        return Err(anyhow!("unexpected bridge read response: {}", response));
    }

    let values = extract_field_value(response, "values")
        .ok_or_else(|| anyhow!("bridge read result has no values field: {}", response))?;
    let mut items = Vec::new();
    for object in split_top_level_objects(values)? {
        let native_tag_id = extract_field_value(object, "tagId")
            .and_then(|value| value.parse::<i32>().ok())
            .ok_or_else(|| anyhow!("bridge read item has no tagId: {}", object))?;
        let quality = extract_field_value(object, "quality")
            .and_then(parse_json_string)
            .ok_or_else(|| anyhow!("bridge read item has no quality: {}", object))?;
        let value = extract_field_value(object, "value")
            .and_then(parse_bridge_value)
            .ok_or_else(|| anyhow!("bridge read item has invalid value: {}", object))?;
        items.push(BridgeReadValue {
            native_tag_id,
            quality,
            value,
        });
    }
    Ok(items)
}

fn split_top_level_objects(values: &str) -> Result<Vec<&str>> {
    let trimmed = values.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Err(anyhow!("expected JSON array: {}", values));
    }

    let mut objects = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (index, ch) in trimmed.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some(start) = start.take() {
                        objects.push(&trimmed[start..=index]);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(objects)
}

fn extract_field_value<'a>(source: &'a str, field_name: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", field_name);
    let field_start = source.find(&needle)?;
    let colon = source[field_start + needle.len()..].find(':')? + field_start + needle.len();
    let whitespace_len = source[colon + 1..]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let value_start = colon + 1 + whitespace_len;
    let first = source[value_start..].chars().next()?;

    match first {
        '"' => {
            let mut escaped = false;
            for (offset, ch) in source[value_start + 1..].char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }
                match ch {
                    '\\' => escaped = true,
                    '"' => return Some(&source[value_start..=value_start + 1 + offset]),
                    _ => {}
                }
            }
            None
        }
        '[' => extract_bracketed_value(source, value_start, '[', ']'),
        '{' => extract_bracketed_value(source, value_start, '{', '}'),
        _ => {
            let end = source[value_start..]
                .find([',', '}', ']'])
                .map(|offset| value_start + offset)
                .unwrap_or(source.len());
            Some(source[value_start..end].trim())
        }
    }
}

fn extract_bracketed_value<'a>(source: &'a str, start: usize, open: char, close: char) -> Option<&'a str> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (offset, ch) in source[start..].char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(&source[start..=start + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_json_string(value: &str) -> Option<String> {
    let inner = value.trim().strip_prefix('"')?.strip_suffix('"')?;
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next()? {
            '\\' => result.push('\\'),
            '"' => result.push('"'),
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            other => result.push(other),
        }
    }
    Some(result)
}

fn parse_bridge_value(value: &str) -> Option<BridgeValue> {
    let trimmed = value.trim();
    if trimmed.starts_with('"') {
        return parse_json_string(trimmed).map(BridgeValue::String);
    }
    if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false") {
        return trimmed.parse::<bool>().ok().map(BridgeValue::Bool);
    }
    trimmed.parse::<f64>().ok().map(BridgeValue::Number)
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_candidates_prioritize_driver_ui_and_x86_target_paths() {
        let candidates = bridge_exe_candidates();

        let driver_ui_index = candidates
            .iter()
            .position(|path| path.ends_with(Path::new(r"driver-ui\joywatcher\joywatcher-bridge-x86.exe")))
            .expect("driver-ui candidate should exist");
        let x86_debug_index = candidates
            .iter()
            .position(|path| path.ends_with(Path::new(r"target\i686-pc-windows-msvc\debug\joywatcher-bridge-x86.exe")))
            .expect("x86 target candidate should exist");
        let x64_debug_index = candidates
            .iter()
            .position(|path| path.ends_with(Path::new(r"target\debug\joywatcher-bridge-x86.exe")));

        assert!(driver_ui_index < x86_debug_index || driver_ui_index == x86_debug_index.saturating_sub(1));
        if let Some(x64_debug_index) = x64_debug_index {
            assert!(x86_debug_index < x64_debug_index);
        }
    }

    #[test]
    fn parse_read_result_decodes_number_string_and_bool_values() {
        let values = parse_read_result(
            r#"{"type":"readResult","request_id":"r1","values":[{"tagId":10,"quality":"good","value":12.5},{"tagId":11,"quality":"bad","value":"ERR"},{"tagId":12,"quality":"good","value":true}]}"#,
        )
        .unwrap();

        assert_eq!(values.len(), 3);
        assert_eq!(values[0].value, BridgeValue::Number(12.5));
        assert_eq!(values[1].value, BridgeValue::String("ERR".to_string()));
        assert_eq!(values[2].value, BridgeValue::Bool(true));
    }
}
