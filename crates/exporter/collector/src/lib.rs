//! Exporter sending events to a Collector service

use quent_collector::{
    client::Client, default::QUENT_COLLECTOR_PORT, env::QUENT_COLLECTOR_ADDRESS,
};
use quent_events::{Event, EventData};
use quent_exporter::{Exporter, ExporterError, ExporterResult};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct CollectorExporterOptions {
    pub address: Option<String>,
}

#[derive(Debug)]
pub struct CollectorExporter {
    client: Client,
}

impl CollectorExporter {
    pub async fn new(
        engine_id: Uuid,
        options: CollectorExporterOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let address = options.address.unwrap_or(
            std::env::var(QUENT_COLLECTOR_ADDRESS)
                .unwrap_or_else(|_| format!("http://[::]:{}", QUENT_COLLECTOR_PORT)),
        );
        let client = Client::new(engine_id, address).await?;
        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl Exporter for CollectorExporter {
    async fn push(&self, event: Event<EventData>) -> ExporterResult<()> {
        self.client
            .send(event)
            .await
            .map_err(|e| ExporterError::Collector(format!("{e:?}")))?;
        Ok(())
    }
    async fn force_flush(&self) -> ExporterResult<()> {
        // TODO(johanpel): figure this out, it may be that we don't need this trait fn
        Ok(())
    }
}
