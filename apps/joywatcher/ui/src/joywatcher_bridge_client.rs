use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::path_utils::{BRIDGE_EXE_NAME, DLL_FILE_NAME};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbedTagType {
    pub tag_id: i32,
    pub value_kind: String,
    pub quality: String,
    pub dtype: Option<i32>,
}

pub fn resolve_single_tag(
    endpoint: &str,
    user_id: i32,
    password: &str,
    tag_path: &str,
) -> Result<i32, String> {
    let mut bridge = JoyWatcherUiBridgeClient::start()?;
    bridge.ping()?;
    bridge.connect(endpoint, user_id, password)?;
    bridge.resolve_tag(tag_path)
}

pub fn browse_tags(endpoint: &str, user_id: i32, password: &str) -> Result<Vec<String>, String> {
    let mut bridge = JoyWatcherUiBridgeClient::start()?;
    bridge.ping()?;
    bridge.connect(endpoint, user_id, password)?;
    let tag_paths = bridge.browse_tags()?;
    if tag_paths.is_empty() {
        return Ok(Vec::new());
    }

    let resolved = bridge.resolve_tags(&tag_paths)?;
    Ok(resolved
        .into_iter()
        .map(|(tag_path, tag_id)| format!("{}|{}", tag_path, tag_id))
        .collect())
}

pub fn probe_tag_types(
    endpoint: &str,
    user_id: i32,
    password: &str,
    tag_ids: &[i32],
) -> Result<Vec<ProbedTagType>, String> {
    if tag_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut bridge = JoyWatcherUiBridgeClient::start()?;
    bridge.ping()?;
    bridge.connect(endpoint, user_id, password)?;
    bridge.read_tag_types(tag_ids)
}

struct JoyWatcherUiBridgeClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    connected: bool,
}

impl JoyWatcherUiBridgeClient {
    fn start() -> Result<Self, String> {
        let exe_path = resolve_bridge_exe_path()
            .ok_or_else(|| format!("JoyWatcher bridge executable not found: {BRIDGE_EXE_NAME}"))?;
        let dll_path = resolve_dll_path()
            .ok_or_else(|| format!("JoyWaApi.dll not found in known search roots: {DLL_FILE_NAME}"))?;

        let mut command = Command::new(&exe_path);
        command
            .arg("--mode")
            .arg("dll")
            .arg("--dll-path")
            .arg(&dll_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = command
            .spawn()
            .map_err(|e| format!("failed to spawn JoyWatcher bridge '{}': {e}", exe_path.display()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to capture JoyWatcher bridge stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "failed to capture JoyWatcher bridge stdout".to_string())?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            connected: false,
        })
    }

    fn ping(&mut self) -> Result<(), String> {
        let response = self.send_request(r#"{"type":"ping"}"#)?;
        if !response.contains(r#""type":"pong""#) {
            return Err(format!("unexpected bridge ping response: {response}"));
        }
        Ok(())
    }

    fn connect(&mut self, endpoint: &str, user_id: i32, password: &str) -> Result<(), String> {
        let response = self.send_request(&format!(
            "{{\"type\":\"connect\",\"endpoint\":\"{}\",\"user_id\":{},\"password\":\"{}\"}}",
            escape_json(endpoint),
            user_id,
            escape_json(password)
        ))?;
        if !response.contains(r#""type":"connected""#) {
            return Err(describe_bridge_response("connect", &response));
        }
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), String> {
        let response = self.send_request(r#"{"type":"disconnect"}"#)?;
        if !response.contains(r#""type":"disconnected""#) {
            return Err(describe_bridge_response("disconnect", &response));
        }
        self.connected = false;
        Ok(())
    }

    fn resolve_tag(&mut self, tag_path: &str) -> Result<i32, String> {
        let response = self.send_request(&format!(
            "{{\"type\":\"resolveTags\",\"tags\":[\"{}\"]}}",
            escape_json(tag_path)
        ))?;

        if !response.contains(r#""type":"resolvedTags""#) {
            return Err(describe_bridge_response("resolveTags", &response));
        }

        extract_i32_field(&response, "\"tagId\":")
            .ok_or_else(|| format!("tagId not found in bridge response: {response}"))
    }

    fn browse_tags(&mut self) -> Result<Vec<String>, String> {
        let response = self.send_request(r#"{"type":"browseTags"}"#)?;

        if !response.contains(r#""type":"browsedTags""#) {
            return Err(describe_bridge_response("browseTags", &response));
        }

        extract_string_array_field(&response, r#""items":["#)
            .ok_or_else(|| format!("items not found in bridge response: {response}"))
    }

    fn resolve_tags(&mut self, tag_paths: &[String]) -> Result<Vec<(String, i32)>, String> {
        let joined = tag_paths
            .iter()
            .map(|tag_path| format!("\"{}\"", escape_json(tag_path)))
            .collect::<Vec<_>>()
            .join(",");
        let response = self.send_request(&format!(
            "{{\"type\":\"resolveTags\",\"tags\":[{}]}}",
            joined
        ))?;

        if !response.contains(r#""type":"resolvedTags""#) {
            return Err(describe_bridge_response("resolveTags", &response));
        }

        extract_resolved_tags(&response)
            .ok_or_else(|| format!("resolved tag items not found in bridge response: {response}"))
    }

    fn read_tag_types(&mut self, tag_ids: &[i32]) -> Result<Vec<ProbedTagType>, String> {
        let joined = tag_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let response = self.send_request(&format!(
            "{{\"type\":\"read\",\"request_id\":\"ui-type-probe\",\"tag_ids\":[{}]}}",
            joined
        ))?;

        if !response.contains(r#""type":"readResult""#) {
            return Err(describe_bridge_response("read", &response));
        }

        extract_probed_tag_types(&response)
            .ok_or_else(|| format!("read values not found in bridge response: {response}"))
    }

    fn send_request(&mut self, request: &str) -> Result<String, String> {
        writeln!(self.stdin, "{request}")
            .map_err(|e| format!("failed to write bridge request: {e}"))?;
        self.stdin
            .flush()
            .map_err(|e| format!("failed to flush bridge stdin: {e}"))?;

        loop {
            let mut line = String::new();
            let bytes = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| format!("failed to read bridge response: {e}"))?;
            if bytes == 0 {
                return Err("bridge returned EOF before responding".to_string());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                continue;
            }

            return Ok(trimmed.to_string());
        }
    }
}

impl Drop for JoyWatcherUiBridgeClient {
    fn drop(&mut self) {
        if self.connected {
            let _ = self.disconnect();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn describe_bridge_response(action: &str, response: &str) -> String {
    if let Some(message) = extract_string_field(response, r#""message":"#) {
        return format!("bridge {action} failed: {message}");
    }
    format!("unexpected bridge {action} response: {response}")
}

fn extract_i32_field(text: &str, key: &str) -> Option<i32> {
    let start = text.find(key)? + key.len();
    let value = &text[start..];
    let end = value
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(value.len());
    value[..end].parse().ok()
}

fn extract_string_field(text: &str, key: &str) -> Option<String> {
    let start = text.find(key)? + key.len();
    let value = text[start..].strip_prefix('"').unwrap_or(&text[start..]);
    let end = value.find('"')?;
    Some(value[..end].replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn extract_string_array_field(text: &str, key: &str) -> Option<Vec<String>> {
    let start = text.find(key)? + key.len();
    let end = text[start..].find(']')? + start;
    let body = &text[start..end];

    if body.is_empty() {
        return Some(Vec::new());
    }

    Some(
        body.split("\",\"")
            .map(|item| item.trim_matches('"').replace("\\\"", "\"").replace("\\\\", "\\"))
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn extract_resolved_tags(text: &str) -> Option<Vec<(String, i32)>> {
    let marker = r#""items":[{"#;
    let start = text.find(marker)? + marker.len() - 1;
    let end = text[start..].rfind(']')? + start;
    let body = &text[start..end];

    let mut items = Vec::new();
    for chunk in body.split("},{") {
        let tag_path = extract_string_field(chunk, r#""tagPath":"#)?;
        let tag_id = extract_i32_field(chunk, "\"tagId\":")?;
        items.push((tag_path, tag_id));
    }

    Some(items)
}

fn extract_probed_tag_types(text: &str) -> Option<Vec<ProbedTagType>> {
    let values = extract_field_value(text, "values")?;
    let mut items = Vec::new();

    for chunk in split_top_level_objects(values)? {
        let tag_id = extract_i32_field(chunk, "\"tagId\":")?;
        let quality = extract_string_field(chunk, r#""quality":"#)?;
        let value = extract_field_value(chunk, "value")?;
        let value_kind = infer_value_kind(value)?;
        let dtype = extract_i32_field(chunk, "\"dtype\":");
        items.push(ProbedTagType {
            tag_id,
            quality,
            value_kind,
            dtype,
        });
    }

    Some(items)
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

fn split_top_level_objects(values: &str) -> Option<Vec<&str>> {
    let trimmed = values.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return None;
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

    Some(objects)
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

fn infer_value_kind(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.starts_with('"') {
        return Some("string".to_string());
    }
    if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false") {
        return Some("bool".to_string());
    }
    if trimmed.parse::<f64>().is_ok() {
        return Some("number".to_string());
    }
    None
}

fn escape_json(value: &str) -> String {
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

fn resolve_bridge_exe_path() -> Option<PathBuf> {
    crate::path_utils::bridge_exe_candidates()
        .into_iter()
        .find(|path| path.exists())
}

fn resolve_dll_path() -> Option<PathBuf> {
    crate::path_utils::dll_file_candidates()
        .into_iter()
        .find(|path| path.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tag_id_from_resolved_response() {
        let response = r#"{"type":"resolvedTags","items":[{"tagPath":"Line1/Tank/Level","tagId":101}]}"#;
        assert_eq!(extract_i32_field(response, "\"tagId\":"), Some(101));
    }

    #[test]
    fn extract_bridge_error_message() {
        let response = r#"{"type":"error","code":"X","message":"resolve failed"}"#;
        assert_eq!(
            describe_bridge_response("resolveTags", response),
            "bridge resolveTags failed: resolve failed"
        );
    }

    #[test]
    fn extract_string_array_field_reads_items() {
        let response = r#"{"type":"browsedTags","items":["Line1/Tank/Level","Line1/Tank/Temp"]}"#;
        assert_eq!(
            extract_string_array_field(response, r#""items":["#),
            Some(vec![
                "Line1/Tank/Level".to_string(),
                "Line1/Tank/Temp".to_string()
            ])
        );
    }

    #[test]
    fn extract_resolved_tags_reads_items() {
        let response = r#"{"type":"resolvedTags","items":[{"tagPath":"LOCAL$REPORT.NONAME4$VALUE","tagId":101},{"tagPath":"LOCAL$BTE1B.B0408$VALUE","tagId":102}]}"#;
        assert_eq!(
            extract_resolved_tags(response),
            Some(vec![
                ("LOCAL$REPORT.NONAME4$VALUE".to_string(), 101),
                ("LOCAL$BTE1B.B0408$VALUE".to_string(), 102),
            ])
        );
    }

    #[test]
    fn extract_probed_tag_types_reads_bool_string_and_number() {
        let response = r#"{"type":"readResult","request_id":"ui-type-probe","values":[{"tagId":1,"quality":"good","value":true},{"tagId":2,"quality":"good","value":"ABC"},{"tagId":3,"quality":"good","value":12.5}]}"#;
        assert_eq!(
            extract_probed_tag_types(response),
            Some(vec![
                ProbedTagType {
                    tag_id: 1,
                    quality: "good".to_string(),
                    value_kind: "bool".to_string(),
                    dtype: None,
                },
                ProbedTagType {
                    tag_id: 2,
                    quality: "good".to_string(),
                    value_kind: "string".to_string(),
                    dtype: None,
                },
                ProbedTagType {
                    tag_id: 3,
                    quality: "good".to_string(),
                    value_kind: "number".to_string(),
                    dtype: None,
                },
            ])
        );
    }
}
