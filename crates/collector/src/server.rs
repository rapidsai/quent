//! A gRPC-based server that collects [`Event`]s from multiple sources and exports them.

use std::{str::FromStr, sync::Arc};

use dashmap::DashMap;
use quent_exporter::Exporter;
use quent_exporter_ndjson::NdjsonExporter;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, warn};
use uuid::Uuid;

use crate::proto;

// Simple service to centralize telemetry from distributed clients
//
// TODO(johanpel): clean up exporter after timeout or engine end.
// TODO(johanpel): exporter config
#[derive(Default, Debug)]
pub struct CollectorService {
    exporters: Arc<DashMap<Uuid, Arc<dyn Exporter>>>,
}

#[tonic::async_trait]
impl proto::collector_server::Collector for CollectorService {
    #[tracing::instrument]
    async fn collect_events(
        &self,
        request: Request<Streaming<proto::CollectEventRequest>>,
    ) -> Result<Response<proto::CollectEventResponse>, Status> {
        // Grab the engine id from the request metadata.
        let engine_id_str = request
            .metadata()
            .get("engine-id")
            .ok_or_else(|| {
                Status::invalid_argument("metadata key \"engine-id\" is not present in request")
            })?
            .to_str()
            .map_err(|e| {
                Status::invalid_argument(format!(
                    "metadata value for \"engine-id\" holds invalid string data: {e}"
                ))
            })?;

        let engine_id = Uuid::from_str(engine_id_str).map_err(|e| {
            Status::invalid_argument(format!(
                "metadata value for key \"engine-id\" is not a UUID: {e}"
            ))
        })?;

        let mut stream = request.into_inner();
        let exporters = Arc::clone(&self.exporters);
        let export_join_handle = tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(request) => {
                        // Get an exporter from the DashMap, or insert it if it doesn't exist.
                        let exporter = if exporters.contains_key(&engine_id) {
                            Arc::clone(&exporters.get(&engine_id).unwrap())
                        } else {
                            let exporter = match NdjsonExporter::try_new(engine_id).await {
                                Ok(exporter) => exporter,
                                Err(e) => {
                                    error!("unable to construct exporter: {e}");
                                    break;
                                }
                            };
                            let exporter: Arc<dyn Exporter> = Arc::new(exporter);
                            exporters.insert(engine_id, Arc::clone(&exporter));
                            exporter
                        };

                        match ciborium::from_reader(std::io::Cursor::new(request.payload)) {
                            Ok(event) => match exporter.push(event).await {
                                Ok(_) => (), // successfully exported
                                Err(e) => {
                                    warn!("collector: unable to export: {e}")
                                }
                            },
                            Err(e) => {
                                warn!("collector: deserialization error: {e}")
                            }
                        }
                    }
                    Err(err) => {
                        warn!("collector: stream error: {err:?}");
                        // TODO(johanpel): a client disconnecting (abruptly?) may result in entering this branch.
                        // We should clean up here, but the todo is to figure out what else can go wrong.
                        if let Some(exporter) = exporters.get(&engine_id) {
                            match exporter.force_flush().await {
                                Ok(_) => (),
                                Err(e) => warn!("unable to flush exporter: {e}"),
                            }
                            exporters.remove(&engine_id);
                        }
                        break;
                    }
                }
            }
        });
        let _ = export_join_handle.await;
        Ok(Response::new(proto::CollectEventResponse {}))
    }
}
