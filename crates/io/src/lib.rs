// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Umbrella crate providing unified exporter/importer creation.

use quent_events::EntityEvent;

use uuid::Uuid;

// Error out compilation if no exporter is selected at all.
#[cfg(not(any(
    feature = "ndjson",
    feature = "msgpack",
    feature = "postcard",
    feature = "collector",
    feature = "callback"
)))]
compile_error!("at least one exporter feature must be enabled");

// Re-exports.
pub use quent_io_types::{
    Exporter, ExporterProvider, ExporterResult, ImporterError, ImporterProvider, ImporterResult,
};

// Feature-gated re-exports for convenience.
#[cfg(filesystem)]
pub use crate::filesystem::{
    Format as FileSystemFormat, exporter::Options as FileSystemExporterOptions,
};
#[cfg(feature = "callback")]
pub use quent_io_callback::EventCallback;
#[cfg(feature = "collector")]
pub use quent_io_collector::Options as CollectorExporterOptions;

// Featue-gated mods.
#[cfg(feature = "clap")]
pub mod clap;
#[cfg(filesystem)]
pub mod filesystem;

/// Where events go: local files (filesystem), a collector service, or a
/// caller-supplied callback (e.g. an in-memory collector for tests).
#[derive(Debug, Clone)]
pub enum ExporterOptions {
    #[cfg(filesystem)]
    FileSystem(FileSystemExporterOptions),
    #[cfg(feature = "collector")]
    Collector(CollectorExporterOptions),
    #[cfg(feature = "callback")]
    Callback(EventCallback),
}

impl ExporterOptions {
    /// Filesystem output directory (`root/<context_id>`) for filesystem
    /// exporters; `None` otherwise. Used to locate the provenance sidecar.
    #[cfg_attr(not(filesystem), allow(unused_variables))]
    pub fn filesystem_root(&self, context_id: Uuid) -> Option<std::path::PathBuf> {
        match self {
            #[cfg(filesystem)]
            ExporterOptions::FileSystem(options) => Some(options.dir(context_id)),
            #[cfg(feature = "collector")]
            ExporterOptions::Collector(_) => None,
            #[cfg(feature = "callback")]
            ExporterOptions::Callback(_) => None,
        }
    }
}

#[cfg(any(filesystem, feature = "collector"))]
#[async_trait::async_trait]
impl<T> ExporterProvider<T> for ExporterOptions
where
    T: serde::Serialize + Send + EntityEvent + 'static,
{
    async fn create_exporter(&self, context_id: Uuid) -> ExporterResult<Box<dyn Exporter<T>>> {
        match self {
            #[cfg(filesystem)]
            ExporterOptions::FileSystem(options) => options.create_exporter(context_id).await,
            #[cfg(feature = "collector")]
            ExporterOptions::Collector(options) => options.create_exporter(context_id).await,
            #[cfg(feature = "callback")]
            ExporterOptions::Callback(callback) => callback.create_exporter(context_id).await,
        }
    }
}

#[cfg(not(any(filesystem, feature = "collector")))]
#[async_trait::async_trait]
impl<T> ExporterProvider<T> for ExporterOptions
where
    T: Send + EntityEvent + 'static,
{
    async fn create_exporter(&self, context_id: Uuid) -> ExporterResult<Box<dyn Exporter<T>>> {
        match self {
            #[cfg(feature = "callback")]
            ExporterOptions::Callback(callback) => callback.create_exporter(context_id).await,
        }
    }
}

/// Selects an importer and its options.
#[derive(Debug, Clone)]
pub enum ImporterOptions {
    #[cfg(filesystem)]
    FileSystem(crate::filesystem::importer::Options),
}

#[cfg(filesystem)]
impl<T> ImporterProvider<T> for ImporterOptions
where
    T: 'static,
    for<'a> T: serde::Deserialize<'a>,
{
    fn create_importer(&self) -> ImporterResult<Box<dyn quent_io_types::Importer<T>>> {
        match self {
            #[cfg(filesystem)]
            ImporterOptions::FileSystem(options) => options.create_importer(),
        }
    }
}
