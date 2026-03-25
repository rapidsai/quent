// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A gRPC-based client that can send [`Event`]s to a collector.

use std::time::Duration;

use quent_events::Event;
use serde::Serialize;
use tokio::{
    runtime::Handle,
    select,
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Status, transport::Channel};

use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use quent_collector_proto::{CollectEventRequest, collector_client::CollectorClient};

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("Unable to connect: {0}")]
    Connect(String),
    #[error("Send error: {0}")]
    SendError(String),
    #[error("Transport error: {0}")]
    Tonic(#[from] tonic::transport::Error),
    #[error("RPC error: {0}")]
    GRPC(#[from] Status),
}

pub type CollectorResult<T> = std::result::Result<T, CollectorError>;

// Trivial implementation of a gRPC client that sends events to a centralized collector
#[derive(Debug)]
pub struct Client<T> {
    _grpc_client: CollectorClient<Channel>,
    event_sender: Sender<Event<T>>,
    cancellation_token: CancellationToken,
    events_sender_handle: Option<JoinHandle<()>>,
    events_collector_handle: Option<JoinHandle<()>>,
    runtime_handle: Handle,
}

impl<T> Client<T>
where
    T: Serialize + Send + 'static,
{
    pub async fn new(application_id: Uuid, address: String) -> CollectorResult<Client<T>> {
        debug!("connecting to {address}");
        // Try to connect.
        // TODO(johanpel): figure out whether this can also go through health check
        const MAX_RETRIES: usize = 42;
        let mut client = Err(CollectorError::Connect(format!(
            "failed to connect after {MAX_RETRIES} attempts..."
        )));
        for retry in 1..MAX_RETRIES + 1 {
            match CollectorClient::connect(address.clone()).await {
                Ok(c) => {
                    client = Ok(c);
                    break;
                }
                Err(e) => {
                    warn!("unable to connect: {e}, retrying in 1s... {retry}/{MAX_RETRIES}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            };
        }
        let client = client?;

        debug!("connected, preparing channels and spawning control thread ...");
        // TODO(johanpel): consider unbounded
        let (event_sender, mut event_receiver): (Sender<Event<T>>, Receiver<Event<T>>) =
            mpsc::channel(1024);
        let (grpc_sender, grpc_receiver): (
            Sender<CollectEventRequest>,
            Receiver<CollectEventRequest>,
        ) = mpsc::channel(1024);

        let cancellation_token = CancellationToken::new();
        let cloned_token = cancellation_token.clone();

        // Spawn a task that takes events, converts them, and sends them as gRPC messages to the collector.
        let events_sender_handle = tokio::spawn(async move {
            // Batch of serialized events.
            let mut buffer = Vec::new();
            // Number of bytes currently in the buffer.
            let mut num_buffer_bytes = 0usize;
            // Interval by which to export even if the buffer isn't full.
            let mut ticker = tokio::time::interval(Duration::from_millis(128));
            // Max bytes in the buffer.
            // gRPC max default is 4 MiB, reserve 256 KiB for overhead.
            const MAX_BUFFER_BYTES: usize = (4 * 1024 * 1024) - (256 * 1024);
            // Reusable serialization buffer so we can re-use the allocation.
            let mut serialized_event = Vec::with_capacity(4096);

            /// function to flush the buffer
            async fn flush_buffer(
                buffer: &mut Vec<Vec<u8>>,
                num_buffer_bytes: &mut usize,
                grpc_sender: &Sender<CollectEventRequest>,
            ) -> Result<(), ()> {
                if buffer.is_empty() {
                    return Ok(());
                }
                let request = CollectEventRequest {
                    event: std::mem::take(buffer),
                };
                *num_buffer_bytes = 0;
                grpc_sender.send(request).await.map_err(|_| ())
            }

            loop {
                select! {
                    Some(event) = event_receiver.recv() => {
                        serialized_event.clear();
                        if let Err(e) = ciborium::into_writer(&event, &mut serialized_event) {
                            error!("unable to serialize event: {e}");
                            continue;
                        }
                        num_buffer_bytes += serialized_event.len();
                        buffer.push(serialized_event.clone());

                        if num_buffer_bytes >= MAX_BUFFER_BYTES {
                            if flush_buffer(&mut buffer, &mut num_buffer_bytes, &grpc_sender).await.is_err() {
                                error!("server disconnected");
                                break;
                            }
                            ticker.reset();
                        }
                    },
                    _ = ticker.tick() => {
                        if flush_buffer(&mut buffer, &mut num_buffer_bytes, &grpc_sender).await.is_err() {
                            error!("server disconnected");
                            break;
                        }
                    },
                    () = cloned_token.cancelled() => {
                        event_receiver.close();
                        // drain events that are buffered
                        while let Some(event) = event_receiver.recv().await {
                            serialized_event.clear();
                            if let Err(e) = ciborium::into_writer(&event, &mut serialized_event) {
                                error!("unable to serialize event: {e}");
                                continue;
                            }
                            buffer.push(serialized_event.clone());
                        }

                        if flush_buffer(&mut buffer, &mut num_buffer_bytes, &grpc_sender).await.is_err() {
                            error!("server disconnected during shutdown");
                        }
                        let pending = grpc_sender.max_capacity() - grpc_sender.capacity();
                        info!(
                            "client shutting down: {pending} gRPC messages pending, flushing..."
                        );
                        // Drop the sender so the gRPC stream receiver sees
                        // the channel is closed and can complete the stream.
                        drop(grpc_sender);
                        break
                    },
                    else => {
                        info!("client shutting down");
                        break
                    }
                }
            }
        });

        debug!("opening stream ...");

        // Add the application id to the metadata of the request, so the collector knows which application this belongs to.
        let mut req = Request::new(ReceiverStream::new(grpc_receiver));
        req.metadata_mut().insert(
            "application-id",
            application_id
                .to_string()
                .parse()
                .expect("valid metadata value"),
        );

        let mut cloned_client = client.clone();
        let events_collector_handle = tokio::spawn(async move {
            let _ = cloned_client.collect_events(req).await;
        });
        debug!("client ready to send events");

        Ok(Client {
            _grpc_client: client,
            event_sender,
            cancellation_token,
            // Safety: this fn must be called from a tokio runtime.
            runtime_handle: Handle::current(),
            events_sender_handle: Some(events_sender_handle),
            events_collector_handle: Some(events_collector_handle),
        })
    }

    /// Send an event to the collector.
    pub async fn send(&self, event: Event<T>) -> CollectorResult<()> {
        // Convert the event into a gRPC message and stream it to the collector.
        self.event_sender
            .send(event)
            .await
            .map_err(|e| CollectorError::SendError(e.to_string()))
    }
}

impl<T> Drop for Client<T> {
    fn drop(&mut self) {
        self.cancellation_token.cancel();

        // Wait for the sender to finish sending the remaining events
        if let Some(join_handle) = self.events_sender_handle.take()
            && let Err(e) = self.runtime_handle.block_on(join_handle)
        {
            warn!("grpc sender task failed: {e}");
        }

        debug!("events_sender_handle completed");

        // Wait for the collector to finish processing the remaining events
        if let Some(join_handle) = self.events_collector_handle.take()
            && let Err(e) = self.runtime_handle.block_on(join_handle)
        {
            warn!("grpc collector task failed: {e}");
        }
        info!("client shut down, all gRPC messages flushed");
    }
}
