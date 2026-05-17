use std::collections::HashMap;

use crate::joywatcher_bridge::{BridgeConnectionSettings, BridgeReadValue, BridgeValue};
use crate::proto::{GetDriverDefinitionResponse, TagValueMessage};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct RuntimeTag {
    pub tag_id: String,
    pub native_tag_id: i32,
}

#[derive(Debug, Clone)]
pub struct JoyWatcherPollPlan {
    pub connection: BridgeConnectionSettings,
    pub tags: Vec<RuntimeTag>,
}

impl JoyWatcherPollPlan {
    pub fn from_definition(definition: &GetDriverDefinitionResponse) -> Self {
        let settings = definition
            .connection
            .as_ref()
            .map(|connection| &connection.settings);

        let connection = BridgeConnectionSettings {
            endpoint: settings.and_then(|map| get_setting(map, &["endpoint", "host", "address"])),
            user_id: settings
                .and_then(|map| get_setting(map, &["userId", "user_id", "uid"]))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or_default(),
            password: settings
                .and_then(|map| get_setting(map, &["password", "passwd"]))
                .unwrap_or_default(),
        };

        let mut tags = Vec::new();
        for tag in &definition.tags {
            if !tag.enabled {
                continue;
            }

            let Some(native_tag_id) = extract_native_tag_id(&tag.driver_spec_json) else {
                warn!("JoyWatcher tag {} has no nativeTagId; skipping", tag.id);
                continue;
            };

            tags.push(RuntimeTag {
                tag_id: tag.id.clone(),
                native_tag_id,
            });
        }

        Self { connection, tags }
    }

    pub fn native_tag_ids(&self) -> Vec<i32> {
        self.tags.iter().map(|tag| tag.native_tag_id).collect()
    }

    pub fn build_messages(&self, values: &[BridgeReadValue]) -> Vec<TagValueMessage> {
        let mut tags_by_native_id = HashMap::<i32, Vec<&RuntimeTag>>::new();
        for tag in &self.tags {
            tags_by_native_id
                .entry(tag.native_tag_id)
                .or_default()
                .push(tag);
        }

        let mut messages = Vec::new();
        for value in values {
            let Some(tags) = tags_by_native_id.get(&value.native_tag_id) else {
                continue;
            };

            for tag in tags {
                messages.push(TagValueMessage {
                    tag_id: tag.tag_id.clone(),
                    value_json: encode_json_value(&value.value),
                    quality: value.quality.clone(),
                    timestamp: String::new(),
                });
            }
        }

        messages
    }
}

fn get_setting(settings: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| settings.get(*key).cloned())
        .filter(|value| !value.trim().is_empty())
}

fn extract_native_tag_id(driver_spec_json: &str) -> Option<i32> {
    extract_i32_field(driver_spec_json, "nativeTagId")
        .or_else(|| extract_i32_field(driver_spec_json, "native_tag_id"))
}

fn extract_i32_field(source: &str, field_name: &str) -> Option<i32> {
    let needle = format!("\"{}\"", field_name);
    let field_start = source.find(&needle)?;
    let colon_start = source[field_start + needle.len()..].find(':')? + field_start + needle.len();
    let value = source[colon_start + 1..]
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
        .collect::<String>();
    value.parse::<i32>().ok()
}

fn encode_json_value(value: &BridgeValue) -> String {
    match value {
        BridgeValue::Bool(value) => value.to_string(),
        BridgeValue::Number(value) => value.to_string(),
        BridgeValue::String(value) => format!("\"{}\"", escape_json_string(value)),
    }
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
    use crate::proto::{ConnectionSettings, TagDef};

    #[test]
    fn extracts_native_tag_id_from_camel_case_driver_spec() {
        let definition = GetDriverDefinitionResponse {
            connection: Some(ConnectionSettings {
                settings: HashMap::new(),
            }),
            scan_groups: Vec::new(),
            tags: vec![TagDef {
                id: "tag-1".to_string(),
                name: "Level".to_string(),
                data_type: "f64".to_string(),
                scan_group_id: "g1".to_string(),
                driver_spec_json: "{\"nativeTagId\":1234}".to_string(),
                enabled: true,
            }],
        };

        let plan = JoyWatcherPollPlan::from_definition(&definition);
        assert_eq!(plan.native_tag_ids(), vec![1234]);
    }

    #[test]
    fn builds_messages_for_matching_native_tag_ids() {
        let plan = JoyWatcherPollPlan {
            connection: BridgeConnectionSettings::default(),
            tags: vec![RuntimeTag {
                tag_id: "tag-1".to_string(),
                native_tag_id: 77,
            }],
        };

        let messages = plan.build_messages(&[BridgeReadValue {
            native_tag_id: 77,
            quality: "good".to_string(),
            value: BridgeValue::Number(9.5),
        }]);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].tag_id, "tag-1");
        assert_eq!(messages[0].value_json, "9.5");
        assert_eq!(messages[0].quality, "good");
    }
}
