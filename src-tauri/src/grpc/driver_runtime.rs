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

            bus.publish(TagValue {
                tag_id: TagId(msg.tag_id),
                value,
                quality,
                timestamp,
            });

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
