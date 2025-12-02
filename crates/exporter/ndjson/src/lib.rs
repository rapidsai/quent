//! Exporter dumping events as newline-delimited JSON objects into a file.
use quent_exporter::Exporter;

pub struct NdjsonExporter {}

#[async_trait::async_trait]
impl Exporter for NdjsonExporter {
    async fn push(
        &self,
        _event: quent_events::Event<quent_events::EventData>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
}
