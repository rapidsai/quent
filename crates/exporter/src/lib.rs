//! Umbrella crate providing unified exporter/importer creation.
//!
//! Concrete exporter/importer types live in their own sub-crates
//! (`quent-exporter-ndjson`, etc.). Shared traits and errors live in
//! `quent-exporter-types`. This crate provides [`ExporterOptions`],
//! [`ImporterOptions`], and factory functions that dispatch to the
//! appropriate sub-crate based on the selected variant.

use std::path::PathBuf;
use std::sync::Arc;

use quent_exporter_types::{Exporter, ExporterError, ExporterResult, Importer, ImporterResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(not(any(
    feature = "ndjson",
    feature = "msgpack",
    feature = "postcard",
    feature = "collector"
)))]
compile_error!("at least one exporter feature must be enabled");

/// Selects which exporter to use, carrying per-exporter options.
#[derive(Debug, Clone)]
pub enum ExporterOptions {
    #[cfg(feature = "ndjson")]
    Ndjson { output_dir: PathBuf },
    #[cfg(feature = "msgpack")]
    Msgpack { output_dir: PathBuf },
    #[cfg(feature = "postcard")]
    Postcard { output_dir: PathBuf },
    #[cfg(feature = "collector")]
    Collector { address: String },
}

/// Selects which importer to use.
#[derive(Debug, Clone)]
pub enum ImporterOptions {
    #[cfg(feature = "ndjson")]
    Ndjson(PathBuf),
    #[cfg(feature = "msgpack")]
    Msgpack(PathBuf),
    #[cfg(feature = "postcard")]
    Postcard(PathBuf),
}

/// Construct an importer from [`ImporterOptions`].
pub fn create_importer<T>(kind: &ImporterOptions) -> ImporterResult<Box<dyn Importer<T>>>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    match kind {
        #[cfg(feature = "ndjson")]
        ImporterOptions::Ndjson(path) => Ok(Box::new(
            quent_exporter_ndjson::NdjsonImporter::try_new(path)?,
        ) as Box<dyn Importer<T>>),
        #[cfg(feature = "msgpack")]
        ImporterOptions::Msgpack(path) => Ok(Box::new(
            quent_exporter_msgpack::MsgpackImporter::try_new(path)?,
        ) as Box<dyn Importer<T>>),
        #[cfg(feature = "postcard")]
        ImporterOptions::Postcard(path) => Ok(Box::new(
            quent_exporter_postcard::PostcardImporter::try_new(path)?,
        ) as Box<dyn Importer<T>>),
    }
}

/// Construct an exporter from [`ExporterOptions`].
pub async fn create_exporter<T>(
    kind: &ExporterOptions,
    application_id: Uuid,
) -> ExporterResult<Arc<dyn Exporter<T>>>
where
    T: Serialize + Send + std::fmt::Debug + 'static,
{
    match kind {
        #[cfg(feature = "ndjson")]
        ExporterOptions::Ndjson { output_dir } => Ok(Arc::new(
            quent_exporter_ndjson::NdjsonExporter::try_new(
                application_id,
                quent_exporter_ndjson::NdjsonExporterOptions {
                    output_dir: output_dir.clone(),
                },
            )
            .await?,
        ) as Arc<dyn Exporter<T>>),
        #[cfg(feature = "msgpack")]
        ExporterOptions::Msgpack { output_dir } => Ok(Arc::new(
            quent_exporter_msgpack::MsgpackExporter::try_new(
                application_id,
                quent_exporter_msgpack::MsgpackExporterOptions {
                    output_dir: output_dir.clone(),
                },
            )
            .await?,
        ) as Arc<dyn Exporter<T>>),
        #[cfg(feature = "postcard")]
        ExporterOptions::Postcard { output_dir } => Ok(Arc::new(
            quent_exporter_postcard::PostcardExporter::try_new(
                application_id,
                quent_exporter_postcard::PostcardExporterOptions {
                    output_dir: output_dir.clone(),
                },
            )
            .await?,
        ) as Arc<dyn Exporter<T>>),
        #[cfg(feature = "collector")]
        ExporterOptions::Collector { address } => Ok(Arc::new(
            quent_exporter_collector::CollectorExporter::try_new(
                application_id,
                quent_exporter_collector::CollectorExporterOptions {
                    address: address.clone(),
                },
            )
            .await
            .map_err(|e| ExporterError::Collector(e.to_string()))?,
        ) as Arc<dyn Exporter<T>>),
    }
}
