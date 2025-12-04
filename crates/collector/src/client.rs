//! A gRPC-base client that can send [`Event`]s to a collector.

use std::{env, time::Duration};

use crate::proto::{CollectEventRequest, collector_client::CollectorClient};
use quent_events::EventData;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;
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
}

impl Client {
    pub async fn new(engine_id: Uuid) -> Result<Client> {
        let addr = env::var(crate::env::QUENT_COLLECTOR_ADDRESS)
            .unwrap_or_else(|_| format!("http://[::]:{}", crate::default::QUENT_COLLECTOR_PORT));
        debug!("connecting to {addr}");

        // Try to connect.
        // TODO(johanpel): figure out whether this can also go through health check
        const MAX_RETRIES: usize = 42;
        let mut client = Err(Error::Connect(format!(
            "failed to connect after {MAX_RETRIES} attempts..."
        )));
        for retry in 1..MAX_RETRIES + 1 {
            match CollectorClient::connect(addr.clone()).await {
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
        let mut client = client?;

        debug!("connected, preparing channels and spawning control thread ...");
        // TODO(johanpel): consider unbounded
        let (event_sender, mut event_receiver): (Sender<Event>, Receiver<Event>) =
            mpsc::channel(1024);
        let (grpc_sender, grpc_receiver): (
            Sender<CollectEventRequest>,
            Receiver<CollectEventRequest>,
        ) = mpsc::channel(1024);

        // Spawn a task that takes events, converts them, and sends them as gRPC messages to the collector.
        tokio::spawn(async move {
            // TODO: probably want to use recv_many + batch if gRPC doesnt do this already
            loop {
                if let Some(event) = event_receiver.recv().await {
                    let payload = serde_json::to_vec(&event);
                    match payload {
                        Ok(payload) => {
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
                        Err(e) => {
                            error!("error serializing: {e}")
                        }
                    };
                } else {
                    info!("client shutting down");
                    break;
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

        let _resp = client.collect_events(req).await?;
        debug!("client ready to send events");

        Ok(Client {
            _grpc_client: client,
            event_sender,
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
