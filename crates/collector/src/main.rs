use tonic::{Request, Response, Status, transport::Server};

use quent_proto::{
    collector_server::{Collector, CollectorServer},
    {SendEventBatchReply, SendEventBatchRequest},
};

#[derive(Debug, Default)]
pub struct CollectorService {}

#[tonic::async_trait]
impl Collector for CollectorService {
    async fn send_event_batch(
        &self,
        request: Request<SendEventBatchRequest>,
    ) -> Result<Response<SendEventBatchReply>, Status> {
        println!("Event batch: {request:?}");
        Ok(Response::new(SendEventBatchReply {}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let collector = CollectorService::default();

    Server::builder()
        .add_service(CollectorServer::new(collector))
        .serve(addr)
        .await?;

    Ok(())
}
