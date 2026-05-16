use crate::app_state::AppState;
use crate::config::ScanGroupConfig;
use crate::core::{DataType, Tag, TagId};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};
use tracing::{info, warn};

pub const DEFAULT_GRPC_ADDR: &str = "127.0.0.1:55051";

pub mod proto {
    tonic::include_proto!("kt_iot_hub.registration");
}

use proto::tag_registration_service_server::{TagRegistrationService, TagRegistrationServiceServer};
use proto::{
    HealthRequest, HealthResponse, UpsertTagRegistrationRequest, UpsertTagRegistrationResponse,
};

#[derive(Clone)]
struct TagRegistrationGrpcService {
    state: AppState,
}

impl TagRegistrationGrpcService {
    fn parse_optional_json(input: &str) -> Result<Option<serde_json::Value>, Status> {
        if input.trim().is_empty() {
            return Ok(None);
        }
        serde_json::from_str::<serde_json::Value>(input)
            .map(Some)
            .map_err(|e| Status::invalid_argument(format!("Invalid JSON: {}", e)))
    }

    fn parse_required_json(input: &str) -> Result<serde_json::Value, Status> {
        if input.trim().is_empty() {
            return Ok(serde_json::json!({}));
        }
        serde_json::from_str::<serde_json::Value>(input)
            .map_err(|e| Status::invalid_argument(format!("Invalid JSON: {}", e)))
    }
}

#[tonic::async_trait]
impl TagRegistrationService for TagRegistrationGrpcService {
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            status: "ok".to_string(),
        }))
    }

    async fn upsert_tag_registration(
        &self,
        request: Request<UpsertTagRegistrationRequest>,
    ) -> Result<Response<UpsertTagRegistrationResponse>, Status> {
        let req = request.into_inner();

        if req.schema_version != 1 {
            return Err(Status::invalid_argument(format!(
                "Unsupported schema_version: {}",
                req.schema_version
            )));
        }
        if req.driver_id.trim().is_empty() {
            return Err(Status::invalid_argument("driver_id is required"));
        }
        if req.tags.is_empty() {
            return Err(Status::invalid_argument("tags must not be empty"));
        }

        let mut group_ids = HashSet::new();
        for group in &req.scan_groups {
            if group.id.trim().is_empty() {
                return Err(Status::invalid_argument("scan_group.id is required"));
            }
            if !group_ids.insert(group.id.clone()) {
                return Err(Status::invalid_argument(format!(
                    "Duplicate scan_group.id: {}",
                    group.id
                )));
            }
        }

        let mut tag_ids = HashSet::new();
        for tag in &req.tags {
            if tag.id.trim().is_empty() {
                return Err(Status::invalid_argument("tag.id is required"));
            }
            if !tag_ids.insert(tag.id.clone()) {
                return Err(Status::invalid_argument(format!("Duplicate tag.id: {}", tag.id)));
            }
            if DataType::from_str(&tag.data_type).is_err() {
                return Err(Status::invalid_argument(format!(
                    "Unsupported data_type for tag {}: {}",
                    tag.id, tag.data_type
                )));
            }
            if tag.scan_group_id.trim().is_empty() {
                return Err(Status::invalid_argument(format!(
                    "scan_group_id is required for tag {}",
                    tag.id
                )));
            }
            if !group_ids.contains(&tag.scan_group_id) {
                return Err(Status::invalid_argument(format!(
                    "Unknown scan_group_id {} for tag {}",
                    tag.scan_group_id, tag.id
                )));
            }
        }

        let new_groups: Vec<ScanGroupConfig> = req
            .scan_groups
            .iter()
            .map(|g| ScanGroupConfig {
                id: g.id.clone(),
                driver: req.driver_id.clone(),
                scan_rate_ms: g.scan_rate_ms,
                schema: if g.schema.is_empty() {
                    None
                } else {
                    Some(g.schema.clone())
                },
                table: if g.table.is_empty() {
                    None
                } else {
                    Some(g.table.clone())
                },
                timestamp_column: if g.timestamp_column.is_empty() {
                    None
                } else {
                    Some(g.timestamp_column.clone())
                },
                node: if g.node.is_empty() {
                    None
                } else {
                    Some(g.node.clone())
                },
            })
            .collect();

        {
            let mut scan_groups = self.state.scan_groups.write().await;
            let new_group_ids: HashSet<String> = new_groups.iter().map(|g| g.id.clone()).collect();
            scan_groups.retain(|g| !(g.driver == req.driver_id && new_group_ids.contains(&g.id)));
            scan_groups.extend(new_groups);
        }

        for t in &req.tags {
            let data_type = DataType::from_str(&t.data_type)
                .map_err(|e| Status::invalid_argument(format!("Invalid data_type: {}", e)))?;
            let driver_spec = Self::parse_required_json(&t.driver_spec_json)?;
            let metadata = Self::parse_optional_json(&t.metadata_json)?;
            let tag = Tag {
                id: TagId(t.id.clone()),
                name: t.name.clone(),
                data_type,
                driver_id: req.driver_id.clone(),
                scan_group_id: t.scan_group_id.clone(),
                driver_spec,
                metadata,
            };
            self.state.registry.insert(tag).await;
        }

        info!(
            "gRPC upsert accepted: driver_id={}, request_id={}, groups={}, tags={}",
            req.driver_id,
            req.request_id,
            req.scan_groups.len(),
            req.tags.len()
        );

        Ok(Response::new(UpsertTagRegistrationResponse {
            success: true,
            message: "accepted".to_string(),
            accepted_scan_groups: req.scan_groups.len() as u32,
            accepted_tags: req.tags.len() as u32,
        }))
    }
}

pub async fn serve(state: AppState, addr: &str, shutdown_rx: oneshot::Receiver<()>) -> anyhow::Result<()> {
    let addr: SocketAddr = addr.parse()?;
    let service = TagRegistrationGrpcService { state };

    info!("Starting TagRegistration gRPC server on {}", addr);
    tonic::transport::Server::builder()
        .add_service(TagRegistrationServiceServer::new(service))
        .serve_with_shutdown(addr, async move {
            let _ = shutdown_rx.await;
            info!("TagRegistration gRPC shutdown signal received");
        })
        .await?;

    warn!("TagRegistration gRPC server stopped");
    Ok(())
}
