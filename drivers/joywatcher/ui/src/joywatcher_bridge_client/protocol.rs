use std::io::{BufRead, Write};

use super::process::JoyWatcherUiBridgeClient;

impl JoyWatcherUiBridgeClient {
    pub(super) fn send_request(&mut self, request: &str) -> Result<String, String> {
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

pub(super) fn describe_bridge_response(action: &str, response: &str) -> String {
    if let Some(message) = extract_string_field(response, r#""message":"#) {
        return format!("bridge {action} failed: {message}");
    }
    format!("unexpected bridge {action} response: {response}")
}

pub(super) fn extract_i32_field(text: &str, key: &str) -> Option<i32> {
    let start = text.find(key)? + key.len();
    let value = &text[start..];
    let end = value
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(value.len());
    value[..end].parse().ok()
}

pub(super) fn extract_string_field(text: &str, key: &str) -> Option<String> {
    let start = text.find(key)? + key.len();
    let value = text[start..].strip_prefix('"').unwrap_or(&text[start..]);
    let end = value.find('"')?;
    Some(value[..end].replace("\\\"", "\"").replace("\\\\", "\\"))
}

pub(super) fn extract_string_array_field(text: &str, key: &str) -> Option<Vec<String>> {
    let start = text.find(key)? + key.len();
    let end = text[start..].find(']')? + start;
    let body = &text[start..end];

    if body.is_empty() {
        return Some(Vec::new());
    }

    Some(
        body.split("\",\"")
            .map(|item| {
                item.trim_matches('"')
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\")
            })
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

pub(super) fn extract_resolved_tags(text: &str) -> Option<Vec<(String, i32)>> {
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

pub(super) fn extract_field_value<'a>(source: &'a str, field_name: &str) -> Option<&'a str> {
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

pub(super) fn split_top_level_objects(values: &str) -> Option<Vec<&str>> {
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

pub(super) fn extract_bracketed_value(
    source: &str,
    start: usize,
    open: char,
    close: char,
) -> Option<&str> {
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

pub(super) fn infer_value_kind(value: &str) -> Option<String> {
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

pub(super) fn escape_json(value: &str) -> String {
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
    fn extract_tag_id_from_resolved_response() {
        let response =
            r#"{"type":"resolvedTags","items":[{"tagPath":"Line1/Tank/Level","tagId":101}]}"#;
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
}
