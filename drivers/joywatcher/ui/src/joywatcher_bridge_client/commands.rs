use super::process::JoyWatcherUiBridgeClient;
use super::protocol::{
    describe_bridge_response, escape_json, extract_field_value, extract_i32_field,
    extract_resolved_tags, extract_string_array_field, extract_string_field, infer_value_kind,
    split_top_level_objects,
};

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

impl JoyWatcherUiBridgeClient {
    pub(super) fn ping(&mut self) -> Result<(), String> {
        let response = self.send_request(r#"{"type":"ping"}"#)?;
        if !response.contains(r#""type":"pong""#) {
            return Err(format!("unexpected bridge ping response: {response}"));
        }
        Ok(())
    }

    pub(super) fn connect(
        &mut self,
        endpoint: &str,
        user_id: i32,
        password: &str,
    ) -> Result<(), String> {
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

    pub(super) fn disconnect(&mut self) -> Result<(), String> {
        let response = self.send_request(r#"{"type":"disconnect"}"#)?;
        if !response.contains(r#""type":"disconnected""#) {
            return Err(describe_bridge_response("disconnect", &response));
        }
        self.connected = false;
        Ok(())
    }

    pub(super) fn resolve_tag(&mut self, tag_path: &str) -> Result<i32, String> {
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

    pub(super) fn browse_tags(&mut self) -> Result<Vec<String>, String> {
        let response = self.send_request(r#"{"type":"browseTags"}"#)?;

        if !response.contains(r#""type":"browsedTags""#) {
            return Err(describe_bridge_response("browseTags", &response));
        }

        extract_string_array_field(&response, r#""items":["#)
            .ok_or_else(|| format!("items not found in bridge response: {response}"))
    }

    pub(super) fn resolve_tags(
        &mut self,
        tag_paths: &[String],
    ) -> Result<Vec<(String, i32)>, String> {
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

    pub(super) fn read_tag_types(&mut self, tag_ids: &[i32]) -> Result<Vec<ProbedTagType>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
