//! A gRPC-based server that collects [`Event`]s from multiple sources and exports them.

use std::sync::Arc;

use dashmap::DashMap;
use quent_events::{Event, EventData};
use quent_exporter::Exporter;
use quent_exporter_ndjson::NdjsonExporter;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;

use crate::proto;

// Simple service to centralize telemetry from distributed clients
//
// TODO(johanpel): clean up exporter after timeout or engine end.
// TODO(johanpel): exporter config
#[derive(Default)]
pub struct CollectorService {
    exporters: Arc<DashMap<Uuid, Arc<dyn Exporter>>>,
}

#[tonic::async_trait]
impl proto::collector_server::Collector for CollectorService {
    async fn collect_events(
        &self,
        request: Request<Streaming<proto::CollectEventRequest>>,
    ) -> Result<Response<proto::CollectEventResponse>, Status> {
        let mut stream = request.into_inner();
        let exporters = Arc::clone(&self.exporters);
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(request) => {
                        match Uuid::parse_str(&request.engine_id) {
                            Ok(engine_id) => {
                                match exporters.entry(engine_id).or_try_insert_with(|| {
                                    // TODO(johanpel): exporter configuration
                                    Ok::<Arc<dyn Exporter>, Box<dyn std::error::Error + Send>>(
                                        Arc::new(NdjsonExporter::new(engine_id)),
                                    )
                                }) {
                                    Ok(exporter) => {
                                        match serde_json::from_slice::<Event<EventData>>(
                                            &request.payload,
                                        ) {
                                            Ok(event) => match exporter.push(event).await {
                                                Ok(_) => (), // successfully exported
                                                Err(e) => {
                                                    eprintln!("collector: unable to export: {e}")
                                                }
                                            },
                                            Err(e) => {
                                                eprintln!("collector: deserialization error: {e}")
                                            }
                                        }
                                    }
                                    Err(e) => eprintln!("collector: unable to spawn exporter: {e}"),
                                }
                            }
                            Err(e) => {
                                eprintln!("collector: received event with invalid engine id: {e}")
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("collector: collect events stream error: {err:?}");
                        break;
                    }
                }
            }
        });
        Ok(Response::new(proto::CollectEventResponse {}))
    }
}
