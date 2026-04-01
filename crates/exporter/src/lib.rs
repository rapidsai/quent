// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Umbrella crate providing unified exporter/importer creation.

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

#[cfg(feature = "collector")]
pub use quent_exporter_collector::CollectorExporterOptions;
#[cfg(feature = "msgpack")]
pub use quent_exporter_msgpack::{MsgpackExporterOptions, MsgpackImporterOptions};
#[cfg(feature = "ndjson")]
pub use quent_exporter_ndjson::{NdjsonExporterOptions, NdjsonImporterOptions};
#[cfg(feature = "postcard")]
pub use quent_exporter_postcard::{PostcardExporterOptions, PostcardImporterOptions};

/// Selects an exporter and its options.
#[derive(Debug, Clone)]
pub enum ExporterOptions {
    #[cfg(feature = "ndjson")]
    Ndjson(NdjsonExporterOptions),
    #[cfg(feature = "msgpack")]
    Msgpack(MsgpackExporterOptions),
    #[cfg(feature = "postcard")]
    Postcard(PostcardExporterOptions),
    #[cfg(feature = "collector")]
    Collector(CollectorExporterOptions),
}

/// Selects an importer and its options.
#[derive(Debug, Clone)]
pub enum ImporterOptions {
    #[cfg(feature = "ndjson")]
    Ndjson(NdjsonImporterOptions),
    #[cfg(feature = "msgpack")]
    Msgpack(MsgpackImporterOptions),
    #[cfg(feature = "postcard")]
    Postcard(PostcardImporterOptions),
}

/// Construct an importer from [`ImporterOptions`].
pub fn create_importer<T>(kind: &ImporterOptions) -> ImporterResult<Box<dyn Importer<T>>>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    match kind {
        #[cfg(feature = "ndjson")]
        ImporterOptions::Ndjson(options) => Ok(Box::new(
            quent_exporter_ndjson::NdjsonImporter::try_new(options)?,
        ) as Box<dyn Importer<T>>),
        #[cfg(feature = "msgpack")]
        ImporterOptions::Msgpack(options) => Ok(Box::new(
            quent_exporter_msgpack::MsgpackImporter::try_new(options)?,
        ) as Box<dyn Importer<T>>),
        #[cfg(feature = "postcard")]
        ImporterOptions::Postcard(options) => Ok(Box::new(
            quent_exporter_postcard::PostcardImporter::try_new(options)?,
        ) as Box<dyn Importer<T>>),
    }
}

/// Construct an exporter from [`ExporterOptions`].
pub async fn create_exporter<T>(
    kind: ExporterOptions,
    application_id: Uuid,
) -> ExporterResult<Arc<dyn Exporter<T>>>
where
    T: Serialize + Send + std::fmt::Debug + 'static,
{
    match kind {
        #[cfg(feature = "ndjson")]
        ExporterOptions::Ndjson(options) => Ok(Arc::new(
            quent_exporter_ndjson::NdjsonExporter::try_new(application_id, options).await?,
        ) as Arc<dyn Exporter<T>>),
        #[cfg(feature = "msgpack")]
        ExporterOptions::Msgpack(options) => Ok(Arc::new(
            quent_exporter_msgpack::MsgpackExporter::try_new(application_id, options).await?,
        ) as Arc<dyn Exporter<T>>),
        #[cfg(feature = "postcard")]
        ExporterOptions::Postcard(options) => Ok(Arc::new(
            quent_exporter_postcard::PostcardExporter::try_new(application_id, options).await?,
        ) as Arc<dyn Exporter<T>>),
        #[cfg(feature = "collector")]
        ExporterOptions::Collector(options) => Ok(Arc::new(
            quent_exporter_collector::CollectorExporter::try_new(application_id, options)
                .await
                .map_err(|e| ExporterError::Collector(e.to_string()))?,
        ) as Arc<dyn Exporter<T>>),
    }
}
