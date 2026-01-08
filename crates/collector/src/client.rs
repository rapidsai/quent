//! A gRPC-base client that can send [`Event`]s to a collector.

use std::time::Duration;

use crate::proto::{CollectEventRequest, collector_client::CollectorClient};
use quent_events::EventData;
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

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to connect: {0}")]
    Connect(String),
    #[error("Send error: {0}")]
    SendError(String),
    #[error("Transport error: {0}")]
    Tonic(#[from] tonic::transport::Error),
    #[error("RPC error: {0}")]
    GRPC(#[from] Status),
}

pub type Result<T> = std::result::Result<T, Error>;

type Event = quent_events::Event<EventData>;

// Trivial implementation of a gRPC client that sends events to a centralized collector
#[derive(Debug)]
pub struct Client {
    _grpc_client: CollectorClient<Channel>,
    event_sender: Sender<Event>,
    cancellation_token: CancellationToken,
    events_sender_handle: Option<JoinHandle<()>>,
    events_collector_handle: Option<JoinHandle<()>>,
    runtime_handle: Handle,
}

impl Client {
    pub async fn new(engine_id: Uuid, address: String) -> Result<Client> {
        debug!("connecting to {address}");
        // Try to connect.
        // TODO(johanpel): figure out whether this can also go through health check
        const MAX_RETRIES: usize = 42;
        let mut client = Err(Error::Connect(format!(
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
        let (event_sender, mut event_receiver): (Sender<Event>, Receiver<Event>) =
            mpsc::channel(1024);
        let (grpc_sender, grpc_receiver): (
            Sender<CollectEventRequest>,
            Receiver<CollectEventRequest>,
        ) = mpsc::channel(1024);

        let cancellation_token = CancellationToken::new();
        let cloned_token = cancellation_token.clone();

        // Spawn a task that takes events, converts them, and sends them as gRPC messages to the collector.
        let events_sender_handle = tokio::spawn(async move {
            loop {
                select! {
                    // TODO: probably want to use recv_many + batch if gRPC doesnt do this already
                    Some(event) = event_receiver.recv() => {
                        let mut payload: Vec<u8> = Vec::with_capacity(4096);
                        if let Err(e) = ciborium::into_writer(&event, &mut payload) {
                            error!("unable to serialize event: {e}");
                            continue;
                        }

                        let event = CollectEventRequest { payload };
                        match grpc_sender.send(event).await {
                            Ok(()) => {
                                // succesfully sent event
                            }
                            Err(_item) => {
                                error!("server disconnected");
                                break;
                            }
                        }
                    },
                    () = cloned_token.cancelled() => {
                        event_receiver.close();
                        // drain events that are buffered
                        while let Some(event) = event_receiver.recv().await {
                            let mut payload: Vec<u8> = Vec::with_capacity(4096);
                            if let Err(e) = ciborium::into_writer(&event, &mut payload) {
                                error!("unable to serialize event: {e}");
                                continue;
                            }

                            let event = CollectEventRequest { payload };
                            match grpc_sender.send(event).await {
                                Ok(_) => {
                                    // succesfully sent event
                                }
                                Err(_item) => {
                                    error!("server disconnected");
                                    break;
                                }
                            }
                        }
                        info!("client shutting down");
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

        // Add the engine id to the metadata of the request, so the collector knows which engine id this all belongs to.
        let mut req = Request::new(ReceiverStream::new(grpc_receiver));
        req.metadata_mut().insert(
            "engine-id",
            engine_id.to_string().parse().expect("valid metadata value"),
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
    pub async fn send(&self, event: Event) -> Result<()> {
        // Convert the event into a gRPC message and stream it to the collector.
        self.event_sender
            .send(event)
            .await
            .map_err(|e| Error::SendError(e.to_string()))
    }
}

impl Drop for Client {
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
        debug!("events_collector_handle completed");
    }
}
