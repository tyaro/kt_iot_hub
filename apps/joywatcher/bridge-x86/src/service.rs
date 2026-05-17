use anyhow::Result;

use crate::connection::ConnectionManager;
use crate::protocol::{BridgeRequest, BridgeResponse};

pub struct JoyWatcherBridgeService {
    connections: ConnectionManager<Box<dyn crate::connection::JoyWatcherBridgeApi>>,
}

impl JoyWatcherBridgeService {
    pub fn new(api: Box<dyn crate::connection::JoyWatcherBridgeApi>) -> Self {
        Self {
            connections: ConnectionManager::new(api),
        }
    }

    pub fn handle_request(&mut self, request: BridgeRequest) -> BridgeResponse {
        match self.try_handle_request(request) {
            Ok(response) => response,
            Err(error) => BridgeResponse::error("BRIDGE_REQUEST_FAILED", error.to_string()),
        }
    }

    fn try_handle_request(&mut self, request: BridgeRequest) -> Result<BridgeResponse> {
        match request {
            BridgeRequest::Ping => Ok(BridgeResponse::Pong),
            BridgeRequest::Connect { .. } => {
                let active_connections = self.connections.connect()?;
                Ok(BridgeResponse::Connected {
                    active_connections,
                    mode: self.connections.api_ref().mode(),
                })
            }
            BridgeRequest::Disconnect => {
                let active_connections = self.connections.disconnect()?;
                Ok(BridgeResponse::Disconnected { active_connections })
            }
            BridgeRequest::ForceDisconnect => {
                let active_connections = self.connections.force_disconnect()?;
                Ok(BridgeResponse::Disconnected { active_connections })
            }
            BridgeRequest::ResolveTags { tags } => {
                self.connections.ensure_connected()?;
                let items = self.connections.api_mut().resolve_tags(&tags)?;
                Ok(BridgeResponse::ResolvedTags { items })
            }
            BridgeRequest::Read {
                request_id,
                tag_ids,
            } => {
                self.connections.ensure_connected()?;
                let values = self.connections.api_ref().read_tags(&tag_ids)?;
                Ok(BridgeResponse::ReadResult { request_id, values })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mock_api::MockJoyWatcherApi;
    use crate::protocol::BridgeRequest;

    use super::*;

    #[test]
    fn ping_returns_pong() {
        let mut service = JoyWatcherBridgeService::new(Box::new(MockJoyWatcherApi::default()));
        let response = service.handle_request(BridgeRequest::Ping);
        assert_eq!(response, BridgeResponse::Pong);
    }

    #[test]
    fn resolve_tags_requires_connection() {
        let mut service = JoyWatcherBridgeService::new(Box::new(MockJoyWatcherApi::default()));
        let response = service.handle_request(BridgeRequest::ResolveTags {
            tags: vec!["Line1/Tank/Level".to_string()],
        });

        match response {
            BridgeResponse::Error { code, .. } => assert_eq!(code, "BRIDGE_REQUEST_FAILED"),
            other => panic!("expected error response, got {other:?}"),
        }
    }

    #[test]
    fn connect_then_read_returns_mock_values() {
        let mut service = JoyWatcherBridgeService::new(Box::new(MockJoyWatcherApi::default()));
        service.handle_request(BridgeRequest::Connect {
            endpoint: None,
            user_id: None,
            password: None,
        });

        let response = service.handle_request(BridgeRequest::Read {
            request_id: "r1".to_string(),
            tag_ids: vec![1010],
        });

        match response {
            BridgeResponse::ReadResult { request_id, values } => {
                assert_eq!(request_id, "r1");
                assert_eq!(values.len(), 1);
            }
            other => panic!("expected read result, got {other:?}"),
        }
    }
}
