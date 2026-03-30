// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A gRPC-based server that collects [`Event`]s from multiple sources and exports them.

use std::{str::FromStr, sync::Arc};

use dashmap::DashMap;
use quent_exporter::{ExporterOptions, create_exporter};
use quent_exporter_types::Exporter;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, warn};
use uuid::Uuid;

use quent_collector_proto as proto;

#[derive(Debug, Clone)]
pub struct CollectorServiceOptions {
    pub exporter: ExporterOptions,
}

// Simple service to centralize telemetry from distributed clients
//
// TODO(johanpel): clean up exporter after timeout or application end.
#[derive(Debug)]
pub struct CollectorService<T> {
    exporters: Arc<DashMap<Uuid, Arc<dyn Exporter<T>>>>,
    exporter: ExporterOptions,
}

impl<T> CollectorService<T> {
    pub fn new(options: CollectorServiceOptions) -> Self {
        Self {
            exporters: Default::default(),
            exporter: options.exporter,
        }
    }
}

#[tonic::async_trait]
impl<T> proto::collector_server::Collector for CollectorService<T>
where
    for<'de> T: Serialize + Deserialize<'de> + Send + std::fmt::Debug + 'static,
{
    #[tracing::instrument]
    async fn collect_events(
        &self,
        request: Request<Streaming<proto::CollectEventRequest>>,
    ) -> Result<Response<proto::CollectEventResponse>, Status> {
        // Grab the application id from the request metadata.
        let application_id_str = request
            .metadata()
            .get("application-id")
            .ok_or_else(|| {
                Status::invalid_argument("metadata key \"engine-id\" is not present in request")
            })?
            .to_str()
            .map_err(|e| {
                Status::invalid_argument(format!(
                    "metadata value for \"engine-id\" holds invalid string data: {e}"
                ))
            })?;

        let application_id = Uuid::from_str(application_id_str).map_err(|e| {
            Status::invalid_argument(format!(
                "metadata value for key \"application-id\" is not a UUID: {e}"
            ))
        })?;

        let mut stream = request.into_inner();
        let exporters = Arc::clone(&self.exporters);
        let exporter_kind = self.exporter.clone();
        let export_join_handle = tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(request) => {
                        // Get an exporter from the DashMap, or insert it if it doesn't exist.
                        let exporter = if exporters.contains_key(&application_id) {
                            Arc::clone(&exporters.get(&application_id).unwrap())
                        } else {
                            let exporter =
                                match create_exporter::<T>(exporter_kind.clone(), application_id)
                                    .await
                                {
                                    Ok(exporter) => exporter,
                                    Err(e) => {
                                        error!("unable to construct exporter: {e}");
                                        break;
                                    }
                                };
                            exporters.insert(application_id, Arc::clone(&exporter));
                            exporter
                        };

                        let mut events = Vec::with_capacity(request.event.len());
                        tracing::trace_span!("deserializing", num_events = request.event.len())
                            .in_scope(|| {
                                for serialized_event in request.event {
                                    match ciborium::from_reader(&serialized_event[..]) {
                                        Ok(event) => events.push(event),
                                        Err(e) => {
                                            warn!("collector: deserialization error: {e}")
                                        }
                                    }
                                }
                            });

                        tracing::trace_span!("exporting")
                            .in_scope(async || {
                                for event in events {
                                    match exporter.push(event).await {
                                        Ok(_) => (), // successfully exported
                                        Err(e) => {
                                            warn!("collector: unable to export: {e}")
                                        }
                                    }
                                }
                            })
                            .await;
                    }
                    Err(err) => {
                        warn!("collector: stream error: {err:?}");
                        // TODO(johanpel): a client disconnecting (abruptly?) may result in entering this branch.
                        // We should clean up here, but the todo is to figure out what else can go wrong.
                        if let Some(exporter) = exporters.get(&application_id) {
                            match exporter.force_flush().await {
                                Ok(_) => (),
                                Err(e) => warn!("unable to flush exporter: {e}"),
                            }
                            exporters.remove(&application_id);
                        }
                        break;
                    }
                }
            }

            // Flush the exporter when stream ends normally
            if let Some(exporter) = exporters.get(&application_id) {
                match exporter.force_flush().await {
                    Ok(_) => (),
                    Err(e) => warn!("unable to flush exporter after stream completion: {e}"),
                }
            }
        });
        let _ = export_join_handle.await;
        Ok(Response::new(proto::CollectEventResponse {}))
    }
}
