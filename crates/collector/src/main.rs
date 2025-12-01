use std::pin::Pin;

use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::{IntoStreamingRequest, Request, Response, Status, Streaming, transport::Server};

use quent_proto::{
    CollectEventRequest, CollectEventResponse,
    collector_server::{Collector, CollectorServer},
};

// Simple service to centralize telemetry from distributed clients
#[derive(Debug, Default)]
pub struct CollectorService {}

#[tonic::async_trait]
impl Collector for CollectorService {
    async fn collect_events(
        &self,
        request: tonic::Request<Streaming<CollectEventRequest>>,
    ) -> Result<tonic::Response<CollectEventResponse>, Status> {
        let mut stream = request.into_inner();
        // let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(v) => {
                        println!("{v:?}");
                    }
                    Err(err) => {
                        eprintln!("collect events stream error: {err:?}");
                        break;
                    }
                }
            }
        });
        Ok(Response::new(CollectEventResponse {}))
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
