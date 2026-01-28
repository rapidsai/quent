//! Basic traits for Exporter implementations

use quent_events::{Event, EventData};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExporterError {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("flush error: {0}")]
    Flush(String),
    #[error("serde error: {0}")]
    Serde(String),
    #[error("collector error: {0}")]
    Collector(String),
}

#[derive(Error, Debug)]
pub enum ImporterError {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type ExporterResult<T> = std::result::Result<T, ExporterError>;
pub type ImporterResult<T> = std::result::Result<T, ImporterError>;

#[async_trait::async_trait]
pub trait Exporter: Send + Sync + std::fmt::Debug {
    async fn push(&self, event: Event<EventData>) -> ExporterResult<()>;
    async fn force_flush(&self) -> ExporterResult<()>;
}
