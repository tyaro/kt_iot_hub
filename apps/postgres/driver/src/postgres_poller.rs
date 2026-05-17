use crate::proto::{GetDriverDefinitionResponse, TagDef, TagValueMessage};
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_postgres::NoTls;
use tracing::{debug, warn};

const BASE_TICK_MS: u64 = 100;
const INITIAL_RECONNECT_BACKOFF_MS: u64 = 1_000;
const MAX_RECONNECT_BACKOFF_MS: u64 = 30_000;

#[derive(Clone, Copy)]
enum DataType {
    Bool,
    I32,
    I64,
    F32,
    F64,
    String,
}

impl DataType {
    fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "bool" => Ok(Self::Bool),
            "i32" => Ok(Self::I32),
            "i64" => Ok(Self::I64),
            "f32" => Ok(Self::F32),
            "f64" => Ok(Self::F64),
            "string" => Ok(Self::String),
            _ => Err(anyhow!("unsupported data_type: {}", s)),
        }
    }

    fn parse_value(self, raw: &str) -> Result<serde_json::Value> {
        match self {
            Self::Bool => Ok(serde_json::json!(raw.parse::<bool>()?)),
            Self::I32 => Ok(serde_json::json!(raw.parse::<i32>()?)),
            Self::I64 => Ok(serde_json::json!(raw.parse::<i64>()?)),
            Self::F32 => Ok(serde_json::json!(raw.parse::<f32>()?)),
            Self::F64 => Ok(serde_json::json!(raw.parse::<f64>()?)),
            Self::String => Ok(serde_json::json!(raw)),
        }
    }
}

#[derive(Clone)]
struct TagRuntime {
    id: String,
    data_type: DataType,
    value_column: String,
}

#[derive(Clone)]
struct GroupRuntime {
    id: String,
    scan_rate_ms: u32,
    schema: Option<String>,
    table: String,
    timestamp_column: Option<String>,
}

pub struct PostgresPoller {
    dsn: String,
    groups: Vec<GroupRuntime>,
    tags_by_group: HashMap<String, Vec<TagRuntime>>,
}

impl PostgresPoller {
    pub fn from_definition(def: GetDriverDefinitionResponse) -> Result<Self> {
        let settings = def
            .connection
            .ok_or_else(|| anyhow!("connection settings are missing"))?
            .settings;

        let host = settings
            .get("host")
            .cloned()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let port = settings
            .get("port")
            .cloned()
            .unwrap_or_else(|| "5432".to_string());
        let database = settings
            .get("database")
            .or_else(|| settings.get("dbname"))
            .cloned()
            .ok_or_else(|| anyhow!("database setting is missing"))?;
        let username = settings
            .get("username")
            .or_else(|| settings.get("user"))
            .cloned()
            .ok_or_else(|| anyhow!("username setting is missing"))?;
        let password = settings.get("password").cloned().unwrap_or_default();
        let ssl_mode = settings
            .get("ssl_mode")
            .cloned()
            .unwrap_or_else(|| "disable".to_string());

        let dsn = format!(
            "host={} port={} dbname={} user={} password={} sslmode={}",
            host, port, database, username, password, ssl_mode
        );

        let groups: Vec<GroupRuntime> = def
            .scan_groups
            .into_iter()
            .filter_map(|g| {
                if g.table.trim().is_empty() {
                    warn!("scan group {} has no table; skipping", g.id);
                    return None;
                }
                Some(GroupRuntime {
                    id: g.id,
                    scan_rate_ms: g.scan_rate_ms,
                    schema: if g.schema.trim().is_empty() {
                        None
                    } else {
                        Some(g.schema)
                    },
                    table: g.table,
                    timestamp_column: if g.timestamp_column.trim().is_empty() {
                        None
                    } else {
                        Some(g.timestamp_column)
                    },
                })
            })
            .collect();

        let mut tags_by_group: HashMap<String, Vec<TagRuntime>> = HashMap::new();
        for tag in def.tags {
            if !tag.enabled {
                continue;
            }
            let value_column = match extract_value_column(&tag) {
                Some(col) => col,
                None => {
                    warn!("tag {} has no value_column; skipping", tag.id);
                    continue;
                }
            };
            let data_type = DataType::parse(&tag.data_type)?;
            tags_by_group
                .entry(tag.scan_group_id.clone())
                .or_default()
                .push(TagRuntime {
                    id: tag.id,
                    data_type,
                    value_column,
                });
        }

        Ok(Self {
            dsn,
            groups,
            tags_by_group,
        })
    }

    pub async fn run(self, tx: mpsc::Sender<TagValueMessage>) -> Result<()> {
        let mut reconnect_backoff = Duration::from_millis(INITIAL_RECONNECT_BACKOFF_MS);

        loop {
            let (client, connection) = match tokio_postgres::connect(&self.dsn, NoTls).await {
                Ok(pair) => {
                    reconnect_backoff = Duration::from_millis(INITIAL_RECONNECT_BACKOFF_MS);
                    pair
                }
                Err(e) => {
                    warn!("PostgreSQL connect failed: {}", e);
                    tokio::time::sleep(reconnect_backoff).await;
                    reconnect_backoff = reconnect_backoff
                        .saturating_mul(2)
                        .min(Duration::from_millis(MAX_RECONNECT_BACKOFF_MS));
                    continue;
                }
            };

            let mut connection_task = tokio::spawn(async move {
                if let Err(e) = connection.await {
                    warn!("PostgreSQL connection task failed: {}", e);
                }
            });

            let mut ticker = tokio::time::interval(Duration::from_millis(BASE_TICK_MS));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut last_polled: HashMap<String, Instant> = HashMap::new();

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let now = Instant::now();

                        for group in &self.groups {
                            let due = last_polled
                                .get(&group.id)
                                .map(|last| now.duration_since(*last) >= Duration::from_millis(group.scan_rate_ms as u64))
                                .unwrap_or(true);
                            if !due {
                                continue;
                            }

                            let group_tags = match self.tags_by_group.get(&group.id) {
                                Some(tags) if !tags.is_empty() => tags,
                                _ => continue,
                            };

                            last_polled.insert(group.id.clone(), now);
                            let sql = match build_group_query(group, group_tags) {
                                Ok(sql) => sql,
                                Err(e) => {
                                    warn!("build query failed for group {}: {}", group.id, e);
                                    continue;
                                }
                            };

                            let rows = match client.query_opt(&sql, &[]).await {
                                Ok(row) => row,
                                Err(e) => {
                                    warn!("query failed for group {}: {}", group.id, e);
                                    continue;
                                }
                            };

                            let Some(row) = rows else {
                                continue;
                            };

                            for (idx, tag) in group_tags.iter().enumerate() {
                                let raw = match row.try_get::<_, Option<String>>(idx) {
                                    Ok(Some(v)) => v,
                                    Ok(None) => {
                                        continue;
                                    }
                                    Err(e) => {
                                        warn!("row get failed for tag {}: {}", tag.id, e);
                                        continue;
                                    }
                                };

                                let value = match tag.data_type.parse_value(&raw) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        warn!("value parse failed for tag {}: {}", tag.id, e);
                                        continue;
                                    }
                                };

                                let msg = TagValueMessage {
                                    tag_id: tag.id.clone(),
                                    value_json: value.to_string(),
                                    quality: "good".to_string(),
                                    timestamp: Utc::now().to_rfc3339(),
                                };

                                if tx.send(msg).await.is_err() {
                                    return Err(anyhow!("gRPC stream sender is closed"));
                                }
                            }
                        }
                    }
                    _ = &mut connection_task => {
                        warn!("PostgreSQL connection ended; reconnecting");
                        break;
                    }
                }
            }

            reconnect_backoff = reconnect_backoff
                .saturating_mul(2)
                .min(Duration::from_millis(MAX_RECONNECT_BACKOFF_MS));
            debug!("reconnect sleep: {:?}", reconnect_backoff);
            tokio::time::sleep(reconnect_backoff).await;
        }
    }
}

fn extract_value_column(tag: &TagDef) -> Option<String> {
    let spec: serde_json::Value = serde_json::from_str(&tag.driver_spec_json).ok()?;
    spec.get("value_column")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .or_else(|| {
            spec.get("valueColumn")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
}

fn build_group_query(group: &GroupRuntime, tags: &[TagRuntime]) -> Result<String> {
    let col_list = tags
        .iter()
        .map(|t| {
            if !is_safe_identifier(&t.value_column) {
                return Err(anyhow!("unsafe value column: {}", t.value_column));
            }
            Ok(format!("{}::text", quote_identifier(&t.value_column)))
        })
        .collect::<Result<Vec<_>>>()?
        .join(", ");

    let relation = qualified_relation(group.schema.as_deref(), &group.table)?;
    if let Some(ts_col) = group.timestamp_column.as_deref() {
        if !is_safe_identifier(ts_col) {
            return Err(anyhow!("unsafe timestamp column: {}", ts_col));
        }
        Ok(format!(
            "SELECT {col_list} FROM {relation} ORDER BY {} DESC LIMIT 1",
            quote_identifier(ts_col)
        ))
    } else {
        Ok(format!("SELECT {col_list} FROM {relation} LIMIT 1"))
    }
}

fn is_safe_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn qualified_relation(schema: Option<&str>, table: &str) -> Result<String> {
    if !is_safe_identifier(table) {
        return Err(anyhow!("unsafe table identifier: {}", table));
    }
    if let Some(schema) = schema {
        if !is_safe_identifier(schema) {
            return Err(anyhow!("unsafe schema identifier: {}", schema));
        }
        return Ok(format!(
            "{}.{}",
            quote_identifier(schema),
            quote_identifier(table)
        ));
    }
    Ok(quote_identifier(table))
}
