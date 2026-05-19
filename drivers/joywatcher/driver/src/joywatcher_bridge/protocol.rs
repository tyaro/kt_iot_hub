use std::io::{BufRead, Write};

use anyhow::{anyhow, Context, Result};

use super::{BridgeReadValue, BridgeValue, JoyWatcherBridgeProcess};

impl JoyWatcherBridgeProcess {
    pub(super) fn send_request(&mut self, request: &str) -> Result<String> {
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

pub(super) fn parse_read_result(response: &str) -> Result<Vec<BridgeReadValue>> {
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

fn extract_bracketed_value<'a>(
    source: &'a str,
    start: usize,
    open: char,
    close: char,
) -> Option<&'a str> {
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

pub(super) fn escape_json_string(value: &str) -> String {
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
