use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::joywatcher_bridge::{
    BridgeConnectionSettings, BridgeReadValue, BridgeValue, JoyWatcherBridgeProcess,
};
use crate::proto::{GetDriverDefinitionResponse, TagValueMessage};

const BASE_TICK_MS: u64 = 100;
const DEFAULT_SCAN_RATE_MS: u32 = 1_000;
const DEFAULT_CONNECTION_USER_ID: i32 = 1;
const CONNECTION_ENDPOINT_KEYS: &[&str] = &["endpoint", "host", "address"];
const CONNECTION_USER_ID_KEYS: &[&str] = &["user_id", "userId", "uid"];
const CONNECTION_PASSWORD_KEYS: &[&str] = &["password", "passwd"];

#[derive(Debug, Clone)]
pub struct RuntimeTag {
    pub tag_id: String,
    pub native_tag_id: i32,
}

#[derive(Debug, Clone)]
pub struct PollGroup {
    pub id: String,
    pub scan_rate_ms: u32,
    pub tags: Vec<RuntimeTag>,
}

#[derive(Debug, Default, Clone)]
pub struct DriverIoTotals {
    pub rx_bytes_total: u64,
    pub tx_bytes_total: u64,
}

impl PollGroup {
    fn effective_scan_rate_ms(&self) -> u32 {
        if self.scan_rate_ms == 0 {
            DEFAULT_SCAN_RATE_MS
        } else {
            self.scan_rate_ms
        }
    }

    fn unique_native_tag_ids(&self) -> Vec<i32> {
        let mut seen = HashSet::new();
        let mut tag_ids = Vec::new();
        for tag in &self.tags {
            if seen.insert(tag.native_tag_id) {
                tag_ids.push(tag.native_tag_id);
            }
        }
        tag_ids
    }

    fn build_messages(&self, values: &[BridgeReadValue], io_totals: &mut DriverIoTotals) -> Vec<TagValueMessage> {
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

            let encoded_value = encode_json_value(&value.value);
            io_totals.rx_bytes_total = io_totals
                .rx_bytes_total
                .saturating_add(encoded_value.as_bytes().len() as u64)
                .saturating_add(value.quality.as_bytes().len() as u64);

            for tag in tags {
                messages.push(TagValueMessage {
                    tag_id: tag.tag_id.clone(),
                    value_json: encoded_value.clone(),
                    quality: value.quality.clone(),
                    timestamp: String::new(),
                    io_rx_bytes_total: io_totals.rx_bytes_total,
                    io_tx_bytes_total: io_totals.tx_bytes_total,
                });
            }
        }

        messages
    }
}

#[derive(Debug, Clone)]
pub struct JoyWatcherPollPlan {
    pub connection: BridgeConnectionSettings,
    pub groups: Vec<PollGroup>,
}

impl JoyWatcherPollPlan {
    pub fn from_definition(definition: &GetDriverDefinitionResponse) -> Self {
        let settings = definition
            .connection
            .as_ref()
            .map(|connection| &connection.settings);

        let connection = BridgeConnectionSettings {
            endpoint: settings.and_then(|map| get_setting(map, CONNECTION_ENDPOINT_KEYS)),
            user_id: settings
                .and_then(|map| get_setting(map, CONNECTION_USER_ID_KEYS))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(DEFAULT_CONNECTION_USER_ID),
            password: settings
                .and_then(|map| get_setting(map, CONNECTION_PASSWORD_KEYS))
                .unwrap_or_default(),
        };

        let mut groups_by_id = HashMap::<String, PollGroup>::new();
        for group in &definition.scan_groups {
            if group.scan_rate_ms == 0 {
                warn!(
                    "JoyWatcher scan group {} has invalid scan_rate_ms=0; fallback to {}ms",
                    group.id,
                    DEFAULT_SCAN_RATE_MS
                );
            }
            groups_by_id.insert(
                group.id.clone(),
                PollGroup {
                    id: group.id.clone(),
                    scan_rate_ms: group.scan_rate_ms,
                    tags: Vec::new(),
                },
            );
        }

        for tag in &definition.tags {
            if !tag.enabled {
                continue;
            }

            let Some(native_tag_id) = extract_native_tag_id(&tag.driver_spec_json) else {
                warn!("JoyWatcher tag {} has no nativeTagId; skipping", tag.id);
                continue;
            };

            let Some(group) = groups_by_id.get_mut(&tag.scan_group_id) else {
                warn!(
                    "JoyWatcher tag {} references unknown scan group {}; skipping",
                    tag.id,
                    tag.scan_group_id
                );
                continue;
            };

            group.tags.push(RuntimeTag {
                tag_id: tag.id.clone(),
                native_tag_id,
            });
        }

        let groups = groups_by_id
            .into_values()
            .filter_map(|group| {
                if group.tags.is_empty() {
                    debug!("JoyWatcher scan group {} has no enabled tags; skipping", group.id);
                    return None;
                }
                Some(group)
            })
            .collect();

        Self { connection, groups }
    }

    pub async fn run(
        &self,
        bridge: &mut JoyWatcherBridgeProcess,
        tx: mpsc::Sender<TagValueMessage>,
    ) -> Result<()> {
        if self.groups.is_empty() {
            return Err(anyhow!("JoyWatcher poll plan has no scan groups with enabled tags"));
        }

        bridge.ensure_connection(&self.connection)?;

        let mut ticker = tokio::time::interval(Duration::from_millis(BASE_TICK_MS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut last_polled = HashMap::<String, Instant>::new();
        let mut io_totals = DriverIoTotals::default();

        loop {
            ticker.tick().await;
            let now = Instant::now();

            for group in &self.groups {
                let scan_rate = Duration::from_millis(group.effective_scan_rate_ms() as u64);
                let due = last_polled
                    .get(&group.id)
                    .map(|last| now.duration_since(*last) >= scan_rate)
                    .unwrap_or(true);
                if !due {
                    continue;
                }

                last_polled.insert(group.id.clone(), now);
                let native_tag_ids = group.unique_native_tag_ids();
                io_totals.tx_bytes_total = io_totals
                    .tx_bytes_total
                    .saturating_add((native_tag_ids.len() * std::mem::size_of::<i32>()) as u64);

                let values = bridge.read_tags(&native_tag_ids)?;
                let messages = group.build_messages(&values, &mut io_totals);
                if messages.is_empty() {
                    debug!("JoyWatcher scan group {} produced no mapped messages", group.id);
                    continue;
                }

                for message in messages {
                    if tx.send(message).await.is_err() {
                        return Err(anyhow!("gRPC stream sender is closed"));
                    }
                }
            }
        }
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
    use crate::proto::{ConnectionSettings, ScanGroupDef, TagDef};

    #[test]
    fn extracts_native_tag_id_from_camel_case_driver_spec() {
        let mut settings = HashMap::new();
        settings.insert("endpoint".to_string(), "localhost".to_string());
        settings.insert("user_id".to_string(), "11".to_string());
        settings.insert("password".to_string(), "pw".to_string());

        let definition = GetDriverDefinitionResponse {
            connection: Some(ConnectionSettings {
                settings,
            }),
            scan_groups: vec![ScanGroupDef {
                id: "g1".to_string(),
                scan_rate_ms: 500,
                schema: String::new(),
                table: String::new(),
                timestamp_column: String::new(),
                node: String::new(),
            }],
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
        assert_eq!(plan.groups.len(), 1);
        assert_eq!(plan.groups[0].unique_native_tag_ids(), vec![1234]);
        assert_eq!(plan.connection.endpoint.as_deref(), Some("localhost"));
        assert_eq!(plan.connection.user_id, 11);
        assert_eq!(plan.connection.password, "pw");
    }

    #[test]
    fn accepts_legacy_camel_case_user_id_key() {
        let mut settings = HashMap::new();
        settings.insert("endpoint".to_string(), "localhost".to_string());
        settings.insert("userId".to_string(), "12".to_string());
        settings.insert("password".to_string(), "pw2".to_string());

        let definition = GetDriverDefinitionResponse {
            connection: Some(ConnectionSettings { settings }),
            scan_groups: vec![ScanGroupDef {
                id: "g1".to_string(),
                scan_rate_ms: 500,
                schema: String::new(),
                table: String::new(),
                timestamp_column: String::new(),
                node: String::new(),
            }],
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
        assert_eq!(plan.connection.user_id, 12);
        assert_eq!(plan.connection.password, "pw2");
    }

    #[test]
    fn builds_messages_for_matching_native_tag_ids() {
        let group = PollGroup {
            id: "g1".to_string(),
            scan_rate_ms: 1000,
            tags: vec![RuntimeTag {
                tag_id: "tag-1".to_string(),
                native_tag_id: 77,
            }],
        };

        let mut io_totals = DriverIoTotals::default();
        let messages = group.build_messages(&[BridgeReadValue {
            native_tag_id: 77,
            quality: "good".to_string(),
            value: BridgeValue::Number(9.5),
        }], &mut io_totals);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].tag_id, "tag-1");
        assert_eq!(messages[0].value_json, "9.5");
        assert_eq!(messages[0].quality, "good");
    }

    #[test]
    fn deduplicates_native_tag_ids_per_group() {
        let group = PollGroup {
            id: "g1".to_string(),
            scan_rate_ms: 1000,
            tags: vec![
                RuntimeTag {
                    tag_id: "tag-1".to_string(),
                    native_tag_id: 77,
                },
                RuntimeTag {
                    tag_id: "tag-2".to_string(),
                    native_tag_id: 77,
                },
            ],
        };

        assert_eq!(group.unique_native_tag_ids(), vec![77]);
    }
}
