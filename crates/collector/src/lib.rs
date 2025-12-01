use quent_events::Event;
use tokio_stream::StreamExt;
use tonic::{Response, Status, Streaming};

use quent_proto::{CollectEventRequest, CollectEventResponse, collector_server::Collector};

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

        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(request) => match serde_json::from_slice::<Event>(&request.payload) {
                        Ok(event) => println!("collector: received event: {event:?}"),
                        Err(e) => eprintln!("collector: deserialization error: {e}"),
                    },
                    Err(err) => {
                        eprintln!("collector: collect events stream error: {err:?}");
                        break;
                    }
                }ßß
            }
        });
        Ok(Response::new(CollectEventResponse {}))
    }
}
