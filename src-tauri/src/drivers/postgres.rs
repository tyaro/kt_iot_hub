// PostgreSQL ドライバ
// スキャングループ（テーブル）単位でタグ値をバッチ取得し Tag Bus に流す

use crate::config::{DriverConfig, ScanGroupConfig};
use crate::core::{DataType, Quality, Tag, TagBus, TagId, TagRegistry, TagValue};
use crate::drivers::Driver;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio_postgres::NoTls;
use tracing::{error, info, warn};

const BASE_POLL_INTERVAL_MS: u64 = 100;
const INITIAL_RECONNECT_BACKOFF_MS: u64 = 1_000;
const MAX_RECONNECT_BACKOFF_MS: u64 = 30_000;

pub struct PostgresDriver {
    id: String,
    config: DriverConfig,
    scan_groups: Vec<ScanGroupConfig>,
    tags: Arc<tokio::sync::RwLock<HashSet<TagId>>>,
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl PostgresDriver {
    pub fn new(config: DriverConfig, scan_groups: Vec<ScanGroupConfig>) -> Self {
        Self {
            id: config.id.clone(),
            config,
            scan_groups,
            tags: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            stop_tx: None,
            task: None,
        }
    }

    fn get_string_setting(&self, key: &str, default: Option<&str>) -> Result<String> {
        self.config
            .settings
            .get(key)
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .or_else(|| default.map(ToString::to_string))
            .ok_or_else(|| anyhow!("Missing driver setting: {}", key))
    }

    fn get_u16_setting(&self, key: &str, default: u16) -> u16 {
        self.config
            .settings
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as u16)
            .unwrap_or(default)
    }

    fn is_safe_identifier(name: &str) -> bool {
        !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    fn quote_identifier(name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }

    fn parse_value(data_type: DataType, raw: &str) -> Result<serde_json::Value> {
        match data_type {
            DataType::Bool => Ok(serde_json::json!(raw.parse::<bool>()?)),
            DataType::I32 => Ok(serde_json::json!(raw.parse::<i32>()?)),
            DataType::I64 => Ok(serde_json::json!(raw.parse::<i64>()?)),
            DataType::F32 => Ok(serde_json::json!(raw.parse::<f32>()?)),
            DataType::F64 => Ok(serde_json::json!(raw.parse::<f64>()?)),
            DataType::String => Ok(serde_json::json!(raw)),
        }
    }

    /// スキャングループの SELECT クエリを構築する
    /// tags_with_cols の順序でカラムを並べ、結果取得は位置インデックスで行う
    fn build_group_query(
        table: &str,
        timestamp_col: Option<&str>,
        tags_with_cols: &[(&Tag, String)],
    ) -> Result<String> {
        if !Self::is_safe_identifier(table) {
            return Err(anyhow!("Unsafe table identifier: {}", table));
        }
        let col_expressions: Result<Vec<String>> = tags_with_cols
            .iter()
            .map(|(_, col)| {
                if !Self::is_safe_identifier(col) {
                    return Err(anyhow!("Unsafe value_column identifier: {}", col));
                }
                Ok(format!("{}::text", Self::quote_identifier(col)))
            })
            .collect();
        let col_list = col_expressions?.join(", ");
        let relation = Self::quote_identifier(table);
        if let Some(ts_col) = timestamp_col {
            if !Self::is_safe_identifier(ts_col) {
                return Err(anyhow!("Unsafe timestamp_column: {}", ts_col));
            }
            Ok(format!(
                "SELECT {col_list} FROM {relation} ORDER BY {} DESC LIMIT 1",
                Self::quote_identifier(ts_col)
            ))
        } else {
            Ok(format!("SELECT {col_list} FROM {relation} LIMIT 1"))
        }
    }

    /// スキャングループの全タグ値をまとめて publish する
    async fn poll_group(
        client: &tokio_postgres::Client,
        bus: &TagBus,
        tags_with_cols: &[(&Tag, String)],
        sql: &str,
    ) {
        match client.query_opt(sql, &[]).await {
            Ok(Some(row)) => {
                for (idx, (tag, col)) in tags_with_cols.iter().enumerate() {
                    match row.try_get::<_, String>(idx) {
                        Ok(raw) => match Self::parse_value(tag.data_type, &raw) {
                            Ok(value) => bus.publish(TagValue::good(tag.id.clone(), value)),
                            Err(e) => {
                                warn!("Parse error tag={} col={}: {}", tag.id.0, col, e);
                                bus.publish(TagValue::bad(tag.id.clone(), serde_json::json!(raw)));
                            }
                        },
                        Err(e) => {
                            warn!("Row get error tag={} col={}: {}", tag.id.0, col, e);
                            bus.publish(TagValue::bad(
                                tag.id.clone(),
                                serde_json::json!(e.to_string()),
                            ));
                        }
                    }
                }
            }
            Ok(None) => {
                for (tag, _) in tags_with_cols {
                    bus.publish(TagValue::new(
                        tag.id.clone(),
                        serde_json::Value::Null,
                        Quality::Uncertain,
                    ));
                }
            }
            Err(e) => {
                warn!("Group query failed: {}", e);
                for (tag, _) in tags_with_cols {
                    bus.publish(TagValue::bad(
                        tag.id.clone(),
                        serde_json::json!(e.to_string()),
                    ));
                }
            }
        }
    }
}

#[async_trait]
impl Driver for PostgresDriver {
    fn id(&self) -> &str {
        &self.id
    }

    fn driver_type(&self) -> &str {
        "postgres"
    }

    async fn start(&mut self, registry: &TagRegistry, bus: &TagBus) -> Result<()> {
        if self.task.is_some() {
            return Ok(());
        }

        let host = self.get_string_setting("host", Some("127.0.0.1"))?;
        let port = self.get_u16_setting("port", 5432);
        let database = self.get_string_setting("database", None)?;
        let username = self.get_string_setting("username", None)?;
        let password = self.get_string_setting("password", Some(""))?;

        let dsn = format!(
            "host={} port={} dbname={} user={} password={}",
            host, port, database, username, password
        );

        let (stop_tx, mut stop_rx) = oneshot::channel();
        let registry = registry.clone();
        let bus = bus.clone();
        let driver_id = self.id.clone();
        let scan_groups = self.scan_groups.clone();
        let selected_tags = self.tags.clone();

        self.task = Some(tokio::spawn(async move {
            info!("PostgresDriver {} started ({} groups)", driver_id, scan_groups.len());
            let mut reconnect_backoff = Duration::from_millis(INITIAL_RECONNECT_BACKOFF_MS);

            loop {
                let (client, connection) = match tokio_postgres::connect(&dsn, NoTls).await {
                    Ok(pair) => {
                        reconnect_backoff = Duration::from_millis(INITIAL_RECONNECT_BACKOFF_MS);
                        pair
                    }
                    Err(e) => {
                        error!("PostgreSQL connect failed ({}): {}", driver_id, e);
                        tokio::select! {
                            _ = &mut stop_rx => { break; }
                            _ = tokio::time::sleep(reconnect_backoff) => {}
                        }
                        reconnect_backoff = reconnect_backoff
                            .saturating_mul(2)
                            .min(Duration::from_millis(MAX_RECONNECT_BACKOFF_MS));
                        continue;
                    }
                };

                let mut conn_task = tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        error!("PostgreSQL connection error: {}", e);
                    }
                });

                let mut ticker =
                    tokio::time::interval(Duration::from_millis(BASE_POLL_INTERVAL_MS));
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                // スキャングループ毎の最終ポーリング時刻
                let mut last_polled: HashMap<String, Instant> = HashMap::new();

                loop {
                    tokio::select! {
                        _ = &mut stop_rx => {
                            info!("PostgresDriver {} stopped", driver_id);
                            conn_task.abort();
                            let _ = conn_task.await;
                            return;
                        }
                        result = &mut conn_task => {
                            match result {
                                Ok(_) => warn!("PostgreSQL conn task ended ({})", driver_id),
                                Err(e) => warn!("PostgreSQL conn task join error ({}): {}", driver_id, e),
                            }
                            break; // 再接続ループへ
                        }
                        _ = ticker.tick() => {
                            // このドライバに属するタグを取得
                            let mut all_tags = registry.list_all().await;
                            all_tags.retain(|t| t.driver_id == driver_id);
                            let selected = selected_tags.read().await.clone();
                            if !selected.is_empty() {
                                all_tags.retain(|t| selected.contains(&t.id));
                            }

                            let now = Instant::now();
                            for group in &scan_groups {
                                // 周期チェック
                                let due = last_polled
                                    .get(&group.id)
                                    .map(|last| now.duration_since(*last)
                                        >= Duration::from_millis(group.scan_rate_ms as u64))
                                    .unwrap_or(true);
                                if !due { continue; }

                                // グループに属するタグを収集
                                let group_tags: Vec<&Tag> = all_tags.iter()
                                    .filter(|t| t.scan_group_id == group.id)
                                    .collect();
                                if group_tags.is_empty() { continue; }

                                // テーブル名を取得
                                let table = match group.table.as_deref() {
                                    Some(t) => t,
                                    None => {
                                        warn!("ScanGroup {} has no table", group.id);
                                        continue;
                                    }
                                };

                                // タグの value_column を収集（設定漏れはスキップ）
                                let tags_with_cols: Vec<(&Tag, String)> = group_tags
                                    .iter()
                                    .filter_map(|tag| {
                                        tag.driver_spec
                                            .get("value_column")
                                            .and_then(|v| v.as_str())
                                            .map(|col| (*tag, col.to_string()))
                                            .or_else(|| {
                                                warn!("Tag {} missing value_column", tag.id.0);
                                                None
                                            })
                                    })
                                    .collect();
                                if tags_with_cols.is_empty() { continue; }

                                last_polled.insert(group.id.clone(), now);

                                match Self::build_group_query(
                                    table,
                                    group.timestamp_column.as_deref(),
                                    &tags_with_cols,
                                ) {
                                    Ok(sql) => {
                                        Self::poll_group(&client, &bus, &tags_with_cols, &sql).await;
                                    }
                                    Err(e) => {
                                        warn!("Query build error group={}: {}", group.id, e);
                                        for (tag, _) in &tags_with_cols {
                                            bus.publish(TagValue::bad(
                                                tag.id.clone(),
                                                serde_json::json!(e.to_string()),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 再接続バックオフ
                reconnect_backoff = reconnect_backoff
                    .saturating_mul(2)
                    .min(Duration::from_millis(MAX_RECONNECT_BACKOFF_MS));
                tokio::select! {
                    _ = &mut stop_rx => {
                        info!("PostgresDriver {} stopped (reconnect wait)", driver_id);
                        break;
                    }
                    _ = tokio::time::sleep(reconnect_backoff) => {}
                }
            }

            info!("PostgresDriver {} task ended", driver_id);
        }));

        self.stop_tx = Some(stop_tx);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        Ok(())
    }

    async fn register_tag(&mut self, tag_id: &TagId) -> Result<()> {
        self.tags.write().await.insert(tag_id.clone());
        Ok(())
    }

    async fn unregister_tag(&mut self, tag_id: &TagId) -> Result<()> {
        self.tags.write().await.remove(tag_id);
        Ok(())
    }
}
