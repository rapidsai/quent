//! Exporter dumping events as newline-delimited JSON objects into a file.
use quent_exporter::Exporter;
use uuid::Uuid;

pub struct NdjsonExporter {
    engine_id: Uuid,
}

impl NdjsonExporter {
    pub fn new(engine_id: Uuid) -> Self {
        Self { engine_id }
    }
}

#[async_trait::async_trait]
impl Exporter for NdjsonExporter {
    async fn push(
        &self,
        event: quent_events::Event<quent_events::EventData>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!(
            "ndjson exporter: engine: {} event: {event:?}",
            self.engine_id
        );
        Ok(())
    }
}
