// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A gRPC server that reproduces each remote source's local output.
//!
//! Each source streams its events tagged with a `source-context-id`; the server
//! runs one local sink per source id and routes those events into it, so it
//! writes the same output the source would write locally.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use quent_collector_client::CollectorSink;
use tokio::sync::OnceCell;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, warn};
use uuid::Uuid;

use quent_collector_proto as proto;

/// A remote source's locally mirrored context. `cell` builds it exactly once
/// even when several of that source's entity streams reach the service
/// together; `open_streams` counts the source's currently open streams so the
/// context can be dropped when the last one closes.
struct MirroredContext<C> {
    cell: Arc<OnceCell<C>>,
    open_streams: usize,
}

/// Mirrored contexts keyed by remote source context id.
type Contexts<C> = RwLock<HashMap<Uuid, MirroredContext<C>>>;

/// Builds a per-source mirrored context (with its observers ready) from its
/// source context id. Injected by the caller because the generic server can't
/// construct a model-specific context. The build is synchronous from here — the
/// context bridges its own async work internally.
type MakeFn<C> = Arc<dyn Fn(Uuid) -> Result<C, String> + Send + Sync>;

/// Decrements a source's open-stream count when a stream ends, including on
/// cancellation (the handler future being dropped mid-stream). When the last
/// stream of a source closes, removes its context and drops it.
struct StreamGuard<'a, C: Send + Sync + 'static> {
    contexts: &'a Contexts<C>,
    source_context_id: Uuid,
}

impl<C: Send + Sync + 'static> Drop for StreamGuard<'_, C> {
    fn drop(&mut self) {
        let mut map = self.contexts.write().unwrap();
        let Some(entry) = map.get_mut(&self.source_context_id) else {
            return;
        };
        entry.open_streams -= 1;
        if entry.open_streams > 0 {
            return;
        }
        let entry = map.remove(&self.source_context_id).expect("entry present");
        drop(map);
        // Dropping the context blocks to flush its observers; that block must
        // not happen on a runtime worker. Drop it on a plain OS thread so the
        // observers take the off-runtime `block_on` path. `entry.cell` is the
        // sole remaining handle here (no other stream is open), so this also
        // drops the context itself.
        std::thread::spawn(move || drop(entry.cell));
    }
}

/// Centralizes telemetry from distributed sources by reproducing each source's
/// local production through a per-source local sink built by `make`.
///
/// `C` is the generated per-model context (implementing [`CollectorSink`] so it
/// can route serialized events into the right observer).
pub struct CollectorService<C> {
    contexts: Contexts<C>,
    make: MakeFn<C>,
}

impl<C> std::fmt::Debug for CollectorService<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectorService").finish_non_exhaustive()
    }
}

impl<C> CollectorService<C> {
    /// `make` builds a per-source local sink (with its observers ready) from its
    /// source context id; it is invoked once per source on the first received
    /// batch.
    pub fn new(make: impl Fn(Uuid) -> Result<C, String> + Send + Sync + 'static) -> Self {
        Self {
            contexts: Default::default(),
            make: Arc::new(make),
        }
    }
}

#[tonic::async_trait]
impl<C> proto::collector_server::Collector for CollectorService<C>
where
    C: CollectorSink + Send + Sync + 'static,
{
    #[tracing::instrument]
    async fn collect_events(
        &self,
        request: Request<Streaming<proto::CollectEventRequest>>,
    ) -> Result<Response<proto::CollectEventResponse>, Status> {
        // The source identifies its stream with the `source-context-id` metadata.
        let source_context_id = request
            .metadata()
            .get("source-context-id")
            .ok_or_else(|| Status::invalid_argument("missing `source-context-id` metadata"))?
            .to_str()
            .ok()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                Status::invalid_argument("`source-context-id` metadata is not a valid UUID")
            })?;

        // The source tags its stream with the entity type so the local context
        // routes each batch to the matching entity observer.
        let entity_type = request
            .metadata()
            .get("entity-type")
            .ok_or_else(|| Status::invalid_argument("missing `entity-type` metadata"))?
            .to_str()
            .map_err(|_| Status::invalid_argument("`entity-type` metadata is not valid UTF-8"))?
            .to_owned();

        // Register this stream against its source, creating the entry on the
        // first stream of the source.
        {
            let mut map = self.contexts.write().unwrap();
            map.entry(source_context_id)
                .or_insert_with(|| MirroredContext {
                    cell: Arc::new(OnceCell::new()),
                    open_streams: 0,
                })
                .open_streams += 1;
        }
        // Drops the source's context once its last stream closes; runs on
        // normal completion and on cancellation alike.
        let _guard = StreamGuard {
            contexts: &self.contexts,
            source_context_id,
        };

        // Run the loop inline (not in a spawned task) so it is cancelled when
        // this RPC's future is dropped, e.g. when the client disconnects.
        let mut stream = request.into_inner();
        while let Some(item) = stream.next().await {
            match item {
                Ok(request) => {
                    // Vend this source's build-once cell; its entry is present
                    // for as long as this stream is open. The lock is held only
                    // for the lookup, never across construction.
                    let cell = {
                        let map = self.contexts.read().unwrap();
                        Arc::clone(&map.get(&source_context_id).expect("entry registered").cell)
                    };

                    // Built exactly once across this source's streams; concurrent
                    // first-touchers await the same construction. `make` is sync
                    // and bridges its own async observer builds internally.
                    let built = cell
                        .get_or_try_init(|| async { (self.make)(source_context_id) })
                        .await;
                    let context = match built {
                        Ok(context) => context,
                        Err(e) => {
                            error!("unable to construct local context: {e}");
                            break;
                        }
                    };

                    tracing::trace_span!("ingesting", num_events = request.event.len()).in_scope(
                        || {
                            for serialized_event in request.event {
                                if let Err(e) = context.ingest(&entity_type, &serialized_event[..])
                                {
                                    warn!("collector: ingest error: {e}");
                                }
                            }
                        },
                    );
                }
                Err(err) => {
                    warn!("collector: stream error: {err:?}");
                    break;
                }
            }
        }
        Ok(Response::new(proto::CollectEventResponse {}))
    }
}
