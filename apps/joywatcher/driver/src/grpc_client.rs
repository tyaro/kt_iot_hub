use crate::proto::driver_runtime_service_client::DriverRuntimeServiceClient;
use crate::proto::{
    GetDriverDefinitionRequest, GetDriverDefinitionResponse, StreamTagValuesAck, TagValueMessage,
};
use anyhow::Result;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;
use tracing::info;

pub struct DriverRuntimeGrpcClient {
    client: DriverRuntimeServiceClient<Channel>,
    driver_id: String,
    driver_kind: String,
}

impl DriverRuntimeGrpcClient {
    pub async fn connect(grpc_addr: &str, driver_id: String, driver_kind: String) -> Result<Self> {
        let endpoint = format!("http://{}", grpc_addr);
        let client = DriverRuntimeServiceClient::connect(endpoint).await?;
        Ok(Self {
            client,
            driver_id,
            driver_kind,
        })
    }

    pub async fn get_driver_definition(&mut self) -> Result<GetDriverDefinitionResponse> {
        let req = GetDriverDefinitionRequest {
            driver_id: self.driver_id.clone(),
            driver_kind: self.driver_kind.clone(),
        };
        let res = self.client.get_driver_definition(req).await?;
        Ok(res.into_inner())
    }

    #[allow(dead_code)]
    pub async fn stream_tag_values(
        mut self,
        rx: mpsc::Receiver<TagValueMessage>,
    ) -> Result<StreamTagValuesAck> {
        let stream = ReceiverStream::new(rx);
        let response = self.client.stream_tag_values(stream).await?;
        let ack = response.into_inner();
        info!("StreamTagValues ack: success={} msg={}", ack.success, ack.message);
        Ok(ack)
    }
}
