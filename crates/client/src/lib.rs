use quent_events::Event;
use quent_proto::{CollectEventRequest, collector_client::CollectorClient};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Status, transport::Channel};

use thiserror::Error;

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

// Trivial implementation of a gRPC client that bgatche
pub struct Client {
    _client: CollectorClient<Channel>,
    event_sender: Sender<Event>,
}

// #[tonic::async_trait]
// impl quent_proto::collector_client::CollectorClient<Channel> for Client {}

impl Client {
    pub async fn new() -> Result<Client> {
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
                            let event = CollectEventRequest { payload };
                            match grpc_sender.send(event).await {
                                Ok(_) => {
                                    eprintln!("successfully sent event");
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
            _client: client,
            event_sender,
        })
    }

    /// Send an event to the collector.
    pub async fn send(&mut self, event: Event) -> Result<()> {
        // Convert the event into a gRPC message and stream it to the collector.
        self.event_sender
            .send(event)
            .await
            .map_err(|e| Error::SendError(e.to_string()))
    }
}
