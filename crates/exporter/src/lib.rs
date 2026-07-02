// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Umbrella crate providing unified exporter/importer creation.

use std::{path::PathBuf, sync::Arc};

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

/// Selects an exporter and its options.
#[derive(Debug, Clone)]
pub enum ExporterOptions {
    FileSystem(FileSystemExporterOptions),
    #[cfg(feature = "collector")]
    Collector(CollectorExporterOptions),
}

/// Serialization format for the filesystem exporter and importer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemFormat {
    #[cfg(feature = "ndjson")]
    Ndjson,
    #[cfg(feature = "msgpack")]
    Msgpack,
    #[cfg(feature = "postcard")]
    Postcard,
}

/// Options for exporting events to the filesystem in the given `format`. Events
/// are written to `root/events.<ext>`.
#[derive(Debug, Clone)]
pub struct FileSystemExporterOptions {
    pub format: FileSystemFormat,
    pub root: PathBuf,
}

impl ExporterOptions {
    /// Returns these options with filesystem output relocated into a
    /// per-context subdirectory `root/<id>/`. Non-filesystem exporters are
    /// returned unchanged.
    pub fn in_context_dir(self, id: Uuid) -> Self {
        match self {
            ExporterOptions::FileSystem(mut options) => {
                options.root = options.root.join(id.to_string());
                ExporterOptions::FileSystem(options)
            }
            #[cfg(feature = "collector")]
            ExporterOptions::Collector(options) => ExporterOptions::Collector(options),
        }
    }
}

/// Selects an importer and its options.
#[derive(Debug, Clone)]
pub enum ImporterOptions {
    FileSystem(FileSystemImporterOptions),
}

/// Options for importing events from the filesystem in the given `format`.
/// `path` is either a directory containing the event file (located by the
/// format's extension) or a direct file path.
#[derive(Debug, Clone)]
pub struct FileSystemImporterOptions {
    pub format: FileSystemFormat,
    pub path: PathBuf,
}

/// Construct an importer from [`ImporterOptions`].
pub fn create_importer<T>(kind: &ImporterOptions) -> ImporterResult<Box<dyn Importer<T>>>
where
    T: for<'de> Deserialize<'de> + 'static,
{
    match kind {
        ImporterOptions::FileSystem(FileSystemImporterOptions { format, path }) => match format {
            #[cfg(feature = "ndjson")]
            FileSystemFormat::Ndjson => {
                Ok(Box::new(quent_exporter_ndjson::NdjsonImporter::try_new(
                    &quent_exporter_ndjson::NdjsonImporterOptions { path: path.clone() },
                )?) as Box<dyn Importer<T>>)
            }
            #[cfg(feature = "msgpack")]
            FileSystemFormat::Msgpack => {
                Ok(Box::new(quent_exporter_msgpack::MsgpackImporter::try_new(
                    &quent_exporter_msgpack::MsgpackImporterOptions { path: path.clone() },
                )?) as Box<dyn Importer<T>>)
            }
            #[cfg(feature = "postcard")]
            FileSystemFormat::Postcard => {
                Ok(Box::new(quent_exporter_postcard::PostcardImporter::try_new(
                    &quent_exporter_postcard::PostcardImporterOptions { path: path.clone() },
                )?) as Box<dyn Importer<T>>)
            }
        },
    }
}

/// Construct an exporter from [`ExporterOptions`]. Filesystem exporters write to
/// `root/events.<ext>`.
pub async fn create_exporter<T>(kind: ExporterOptions) -> ExporterResult<Arc<dyn Exporter<T>>>
where
    T: Serialize + Send + 'static,
{
    match kind {
        ExporterOptions::FileSystem(FileSystemExporterOptions { format, root }) => match format {
            #[cfg(feature = "ndjson")]
            FileSystemFormat::Ndjson => Ok(Arc::new(
                quent_exporter_ndjson::NdjsonExporter::try_new(
                    quent_exporter_ndjson::NdjsonExporterOptions {
                        path: root.join("events.ndjson"),
                    },
                )
                .await?,
            ) as Arc<dyn Exporter<T>>),
            #[cfg(feature = "msgpack")]
            FileSystemFormat::Msgpack => Ok(Arc::new(
                quent_exporter_msgpack::MsgpackExporter::try_new(
                    quent_exporter_msgpack::MsgpackExporterOptions {
                        path: root.join("events.msgpack"),
                    },
                )
                .await?,
            ) as Arc<dyn Exporter<T>>),
            #[cfg(feature = "postcard")]
            FileSystemFormat::Postcard => Ok(Arc::new(
                quent_exporter_postcard::PostcardExporter::try_new(
                    quent_exporter_postcard::PostcardExporterOptions {
                        path: root.join("events.postcard"),
                    },
                )
                .await?,
            ) as Arc<dyn Exporter<T>>),
        },
        #[cfg(feature = "collector")]
        ExporterOptions::Collector(options) => Ok(Arc::new(
            quent_exporter_collector::CollectorExporter::try_new(options)
                .await
                .map_err(|e| ExporterError::Collector(e.to_string()))?,
        ) as Arc<dyn Exporter<T>>),
    }
}
