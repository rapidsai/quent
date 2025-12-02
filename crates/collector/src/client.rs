//! A gRPC-base client that can send [`Event`]s to a collector.

use crate::proto::{CollectEventRequest, collector_client::CollectorClient};
use quent_events::EventData;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Status, transport::Channel};

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum Error {
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
pub struct Client {
    _grpc_client: CollectorClient<Channel>,
    event_sender: Sender<Event>,
}

impl Client {
    pub async fn new(engine_id: Uuid) -> Result<Client> {
        let mut client = CollectorClient::connect("http://[::1]:50051").await?;

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
                            let event = CollectEventRequest {
                                engine_id: engine_id.to_string(),
                                payload,
                            };
                            match grpc_sender.send(event).await {
                                Ok(_) => {
                                    // succesfully sent event
                                }
                                Err(_item) => {
                                    eprintln!("server disconnected");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("error serializing: {e}")
                        }
                    };
                } else {
                    eprintln!("client shutting down");
                    break;
                }
            }
        });

        // Request setting up the stream
        let _resp = client
            .collect_events(ReceiverStream::new(grpc_receiver))
            .await?;

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
