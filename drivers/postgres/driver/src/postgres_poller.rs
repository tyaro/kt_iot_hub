use crate::proto::{GetDriverDefinitionResponse, TagDef, TagValueMessage};
use anyhow::{anyhow, Result};
use chrono::Utc;
use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_postgres::{Config, NoTls};
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
    connection: PostgresConnectionOptions,
    groups: Vec<GroupRuntime>,
    tags_by_group: HashMap<String, Vec<TagRuntime>>,
}

struct PostgresConnectionOptions {
    host: String,
    port: u16,
    database: String,
    username: String,
    password: String,
    tls_enabled: bool,
    tls_ca_path: Option<String>,
    connect_timeout_ms: Option<u64>,
    statement_timeout_ms: Option<u64>,
}

#[derive(Debug, Default)]
struct DriverIoTotals {
    rx_bytes_total: u64,
    tx_bytes_total: u64,
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
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(5432);
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
        let tls_enabled = parse_bool_setting(settings.get("tls_enabled"))
            || settings
                .get("ssl_mode")
                .map(|mode| !mode.trim().is_empty() && !mode.eq_ignore_ascii_case("disable"))
                .unwrap_or(false);
        let tls_ca_path = settings.get("tls_ca_path").cloned();
        let connect_timeout_ms = parse_u64_setting(settings.get("connect_timeout_ms"));
        let statement_timeout_ms = parse_u64_setting(settings.get("statement_timeout_ms"));

        let connection = PostgresConnectionOptions {
            host,
            port,
            database,
            username,
            password,
            tls_enabled,
            tls_ca_path,
            connect_timeout_ms,
            statement_timeout_ms,
        };

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
            connection,
            groups,
            tags_by_group,
        })
    }

    pub async fn run(self, tx: mpsc::Sender<TagValueMessage>) -> Result<()> {
        let mut reconnect_backoff = Duration::from_millis(INITIAL_RECONNECT_BACKOFF_MS);

        loop {
            let (client, mut connection_task) = match connect_client(&self.connection).await {
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

            let mut ticker = tokio::time::interval(Duration::from_millis(BASE_TICK_MS));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut last_polled: HashMap<String, Instant> = HashMap::new();
            let mut io_totals = DriverIoTotals::default();

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

                            io_totals.tx_bytes_total = io_totals
                                .tx_bytes_total
                                .saturating_add(sql.as_bytes().len() as u64);

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

                                io_totals.rx_bytes_total = io_totals
                                    .rx_bytes_total
                                    .saturating_add(raw.as_bytes().len() as u64);

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
                                    io_rx_bytes_total: io_totals.rx_bytes_total,
                                    io_tx_bytes_total: io_totals.tx_bytes_total,
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

fn parse_bool_setting(value: Option<&String>) -> bool {
    value.and_then(|v| v.parse::<bool>().ok()).unwrap_or(false)
}

fn parse_u64_setting(value: Option<&String>) -> Option<u64> {
    value.and_then(|v| v.parse::<u64>().ok())
}

fn build_config(conn: &PostgresConnectionOptions) -> Config {
    let mut config = Config::new();
    config.host(&conn.host);
    config.port(conn.port);
    config.dbname(&conn.database);
    config.user(&conn.username);
    config.password(&conn.password);

    if let Some(connect_timeout_ms) = conn.connect_timeout_ms {
        config.connect_timeout(Duration::from_millis(connect_timeout_ms));
    }

    config
}

fn build_tls_connector(conn: &PostgresConnectionOptions) -> Result<Option<MakeTlsConnector>> {
    if !conn.tls_enabled {
        return Ok(None);
    }

    let mut builder = TlsConnector::builder();

    if let Some(ca_path) = conn
        .tls_ca_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        let pem = fs::read(ca_path)
            .map_err(|e| anyhow!("failed to read PostgreSQL TLS CA file {}: {}", ca_path, e))?;
        let cert = Certificate::from_pem(&pem)
            .map_err(|e| anyhow!("failed to parse PostgreSQL TLS CA file {}: {}", ca_path, e))?;
        builder.add_root_certificate(cert);
    }

    let connector = builder
        .build()
        .map_err(|e| anyhow!("failed to build PostgreSQL TLS connector: {}", e))?;
    Ok(Some(MakeTlsConnector::new(connector)))
}

async fn connect_client(
    conn: &PostgresConnectionOptions,
) -> Result<(tokio_postgres::Client, tokio::task::JoinHandle<()>)> {
    let config = build_config(conn);

    if let Some(tls) = build_tls_connector(conn)? {
        let (client, connection) = config.connect(tls).await?;
        let connection_task = tokio::spawn(async move {
            if let Err(e) = connection.await {
                warn!("PostgreSQL connection task failed: {}", e);
            }
        });

        if let Some(statement_timeout_ms) = conn.statement_timeout_ms {
            client
                .batch_execute(&format!("SET statement_timeout = {}", statement_timeout_ms))
                .await?;
        }

        Ok((client, connection_task))
    } else {
        let (client, connection) = config.connect(NoTls).await?;
        let connection_task = tokio::spawn(async move {
            if let Err(e) = connection.await {
                warn!("PostgreSQL connection task failed: {}", e);
            }
        });

        if let Some(statement_timeout_ms) = conn.statement_timeout_ms {
            client
                .batch_execute(&format!("SET statement_timeout = {}", statement_timeout_ms))
                .await?;
        }

        Ok((client, connection_task))
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
