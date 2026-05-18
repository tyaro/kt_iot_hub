// DriverRuntimeService: ドライバ通信プロセス ↔ 本体間の gRPC サービス実装
// ドライバプロセスは本体のgRPCサーバへクライアントとして接続する

use crate::app_state::AppState;
use crate::core::{Quality, TagId, TagValue};
use chrono::DateTime;
use std::collections::HashMap;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, info, warn};

pub mod proto {
    tonic::include_proto!("kt_iot_hub.driver_runtime");
}

use proto::driver_runtime_service_server::{DriverRuntimeService, DriverRuntimeServiceServer};
use proto::{
    GetDriverDefinitionRequest, GetDriverDefinitionResponse,
    StreamTagValuesAck, TagValueMessage,
};

#[derive(Clone)]
pub struct DriverRuntimeGrpcService {
    state: AppState,
}

impl DriverRuntimeGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    async fn expected_scan_rate_ms(&self, driver_id: &str, scan_group_id: &str) -> Option<u32> {
        let groups = self.state.scan_groups.read().await;
        groups
            .iter()
            .find(|group| group.driver == driver_id && group.id == scan_group_id)
            .map(|group| group.scan_rate_ms)
    }

    async fn update_scan_group_runtime_metric(
        &self,
        driver_id: &str,
        scan_group_id: &str,
        observed_at: chrono::DateTime<chrono::Utc>,
    ) {
        let expected_scan_rate_ms = self.expected_scan_rate_ms(driver_id, scan_group_id).await;
        let key = format!("{}::{}", driver_id, scan_group_id);

        let mut warn_payload: Option<(u32, u64, u32, f64)> = None;
        {
            let mut metrics = self.state.scan_group_runtime_metrics.write().await;
            let entry = metrics.entry(key).or_insert_with(|| {
                crate::app_state::ScanGroupRuntimeMetricState::new(expected_scan_rate_ms)
            });

            if entry.expected_scan_rate_ms.is_none() {
                entry.expected_scan_rate_ms = expected_scan_rate_ms;
            }

            let Some(last_anchor) = entry.last_cycle_anchor_at else {
                entry.last_cycle_anchor_at = Some(observed_at);
                return;
            };

            let delta_ms = observed_at
                .signed_duration_since(last_anchor)
                .num_milliseconds();
            if delta_ms <= 0 {
                return;
            }

            let expected_for_gate = entry.expected_scan_rate_ms.unwrap_or(1000) as i64;
            let min_cycle_ms = (expected_for_gate / 3).max(10);
            if delta_ms < min_cycle_ms {
                return;
            }

            let cycle_ms = delta_ms as u64;
            entry.last_cycle_anchor_at = Some(observed_at);
            entry.last_cycle_ms = Some(cycle_ms);

            let next_avg = match entry.avg_cycle_ms {
                Some(prev) => (prev * 0.8) + (cycle_ms as f64 * 0.2),
                None => cycle_ms as f64,
            };
            entry.avg_cycle_ms = Some(next_avg);

            if entry.cycle_history_ms.len() >= 32 {
                entry.cycle_history_ms.pop_front();
            }
            entry.cycle_history_ms.push_back(cycle_ms);

            let mut sorted = entry.cycle_history_ms.iter().copied().collect::<Vec<_>>();
            sorted.sort_unstable();
            if !sorted.is_empty() {
                let p95_index = ((sorted.len() - 1) as f64 * 0.95).round() as usize;
                entry.p95_cycle_ms = sorted.get(p95_index).copied();
            }

            if let Some(expected) = entry.expected_scan_rate_ms {
                let ratio = ((next_avg - expected as f64) / expected as f64).abs();
                entry.cycle_delta_ratio = Some(ratio);

                if ratio > 0.50 {
                    entry.consecutive_lag_count = entry.consecutive_lag_count.saturating_add(1);
                } else {
                    entry.consecutive_lag_count = 0;
                }

                if entry.consecutive_lag_count >= 5 {
                    let should_warn = entry
                        .last_warn_at
                        .map(|last| observed_at.signed_duration_since(last).num_seconds() >= 10)
                        .unwrap_or(true);
                    if should_warn {
                        entry.last_warn_at = Some(observed_at);
                        warn_payload = Some((expected, cycle_ms, entry.consecutive_lag_count, ratio));
                    }
                }
            }
        }

        if let Some((expected, cycle_ms, consecutive, ratio)) = warn_payload {
            let ratio_pct = ratio * 100.0;
            warn!(
                "Scan group cycle lag detected: driver={} group={} expected={}ms observed={}ms delta={:.1}% consecutive={}",
                driver_id,
                scan_group_id,
                expected,
                cycle_ms,
                ratio_pct,
                consecutive
            );
        }
    }
}

#[tonic::async_trait]
impl DriverRuntimeService for DriverRuntimeGrpcService {
    /// ドライバプロセス起動時に呼ばれる: タグ定義・スキャングループ・接続設定を返す
    async fn get_driver_definition(
        &self,
        request: Request<GetDriverDefinitionRequest>,
    ) -> Result<Response<GetDriverDefinitionResponse>, Status> {
        let req = request.into_inner();
        info!("GetDriverDefinition: driver_id={} kind={}", req.driver_id, req.driver_kind);

        // ドライバ設定を検索
        let driver_configs = self.state.driver_configs.read().await;
        let driver_config = driver_configs
            .iter()
            .find(|d| d.id == req.driver_id && d.driver_type == req.driver_kind)
            .ok_or_else(|| {
                Status::not_found(format!("Driver not found: {}", req.driver_id))
            })?;

        // 接続設定をstring mapに変換
        let mut settings = HashMap::new();
        if let Some(obj) = driver_config.settings.as_object() {
            for (k, v) in obj {
                let v_str = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                settings.insert(k.clone(), v_str);
            }
        }
        let connection = Some(proto::ConnectionSettings { settings });

        // このドライバに属するスキャングループを取得
        let scan_groups_cfg = self.state.scan_groups.read().await;
        let scan_groups: Vec<proto::ScanGroupDef> = scan_groups_cfg
            .iter()
            .filter(|g| g.driver == req.driver_id)
            .map(|g| proto::ScanGroupDef {
                id: g.id.clone(),
                scan_rate_ms: g.scan_rate_ms,
                schema: g.schema.clone().unwrap_or_default(),
                table: g.table.clone().unwrap_or_default(),
                timestamp_column: g.timestamp_column.clone().unwrap_or_default(),
                node: g.node.clone().unwrap_or_default(),
            })
            .collect();

        // このドライバに属するタグを取得
        let all_tags = self.state.registry.list_all().await;
        let tags: Vec<proto::TagDef> = all_tags
            .into_iter()
            .filter(|t| t.driver_id == req.driver_id)
            .map(|t| proto::TagDef {
                id: t.id.0.clone(),
                name: t.name.clone(),
                data_type: t.data_type.as_str().to_string(),
                scan_group_id: t.scan_group_id.clone(),
                driver_spec_json: serde_json::to_string(&t.driver_spec).unwrap_or_default(),
                enabled: true,
            })
            .collect();

        debug!(
            "GetDriverDefinition: scan_groups={}, tags={}",
            scan_groups.len(),
            tags.len()
        );

        Ok(Response::new(GetDriverDefinitionResponse {
            connection,
            scan_groups,
            tags,
        }))
    }

    /// ドライバプロセスがタグ値をストリーム送信する → Tag Bus に流す
    async fn stream_tag_values(
        &self,
        request: Request<Streaming<TagValueMessage>>,
    ) -> Result<Response<StreamTagValuesAck>, Status> {
        let mut stream = request.into_inner();
        let bus = self.state.tag_bus.clone();
        let mut count = 0u64;

        loop {
            let msg = match stream.message().await {
                Ok(Some(m)) => m,
                Ok(None) => break,
                Err(e) => {
                    warn!("StreamTagValues: stream error: {}", e);
                    break;
                }
            };

            let value = match serde_json::from_str::<serde_json::Value>(&msg.value_json) {
                Ok(v) => v,
                Err(e) => {
                    warn!("StreamTagValues: invalid value_json for {}: {}", msg.tag_id, e);
                    continue;
                }
            };

            let quality = match msg.quality.as_str() {
                "good" => Quality::Good,
                "uncertain" => Quality::Uncertain,
                _ => Quality::Bad,
            };

            let timestamp = DateTime::parse_from_rfc3339(&msg.timestamp)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let tag_id = msg.tag_id.clone();

            bus.publish(TagValue {
                tag_id: TagId(tag_id.clone()),
                value,
                quality,
                timestamp,
            });

            if let Some(tag) = self.state.registry.get(&TagId(tag_id)).await {
                self
                    .update_scan_group_runtime_metric(&tag.driver_id, &tag.scan_group_id, chrono::Utc::now())
                    .await;
            }

            count += 1;
        }

        info!("StreamTagValues: stream ended, total={}", count);
        Ok(Response::new(StreamTagValuesAck {
            success: true,
            message: format!("Accepted {} values", count),
        }))
    }
}

/// DriverRuntimeServiceServer を組み立てて返す
pub fn build_server(state: AppState) -> DriverRuntimeServiceServer<DriverRuntimeGrpcService> {
    DriverRuntimeServiceServer::new(DriverRuntimeGrpcService::new(state))
}
