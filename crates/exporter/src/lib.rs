//! Basic traits for Exporter implementations

use quent_events::{Event, EventData};

#[async_trait::async_trait]
pub trait Exporter: Send + Sync {
    async fn push(&self, event: Event<EventData>) -> Result<(), Box<dyn std::error::Error>>;
    async fn force_flush(&self) -> Result<(), Box<dyn std::error::Error>>;
}
