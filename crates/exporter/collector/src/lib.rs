//! Exporter sending events to a Collector service

use quent_collector::client::Client;
use quent_events::{Event, EventData};
use quent_exporter::Exporter;
use uuid::Uuid;

pub struct CollectorExporter {
    client: Client,
}

impl CollectorExporter {
    pub async fn new(engine_id: Uuid) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::new(engine_id).await?;
        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl Exporter for CollectorExporter {
    async fn push(&self, event: Event<EventData>) -> Result<(), Box<dyn std::error::Error>> {
        self.client.send(event).await?;
        Ok(())
    }
    async fn force_flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO(johanpel): figure this out, it may be that we don't need this trait fn
        Ok(())
    }
}
