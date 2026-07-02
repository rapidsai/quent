// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Umbrella crate providing unified exporter/importer creation.

#[cfg(filesystem)]
use std::path::PathBuf;

use quent_events::EntityEvent;
#[cfg(feature = "collector")]
use quent_exporter_types::ExporterError;
#[cfg(filesystem)]
use quent_exporter_types::Importer;
use quent_exporter_types::{Exporter, ExporterResult};
#[cfg(filesystem)]
use serde::Deserialize;
use serde::Serialize;

#[cfg(feature = "callback")]
pub use quent_exporter_callback::{CallbackExporter, EventCallback, RecordedEvent};

// Part of the public API: `create_importer` returns `ImporterResult`, so callers
// must be able to name it (and its error).
#[cfg(filesystem)]
pub use quent_exporter_types::{ImporterError, ImporterResult};
use uuid::Uuid;

#[cfg(not(any(
    feature = "ndjson",
    feature = "msgpack",
    feature = "postcard",
    feature = "collector",
    feature = "callback"
)))]
compile_error!("at least one exporter feature must be enabled");

#[cfg(feature = "collector")]
pub use quent_exporter_collector::CollectorExporterOptions;

#[cfg(feature = "clap")]
pub mod clap;

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

/// Like [`ExporterOptions`], but the collector variant also carries the source
/// context id and a filesystem `root` is the per-context directory.
#[derive(Debug, Clone)]
pub enum ResolvedExporterOptions {
    #[cfg(filesystem)]
    FileSystem(FileSystemExporterOptions),
    #[cfg(feature = "collector")]
    Collector {
        address: http::Uri,
        source_context_id: Uuid,
    },
    #[cfg(feature = "callback")]
    Callback(EventCallback),
}

impl ResolvedExporterOptions {
    /// Filesystem output directory for filesystem exporters; `None` for
    /// exporters (e.g. the collector) that do not write a local directory.
    /// Used to locate where a provenance sidecar should be written.
    pub fn filesystem_root(&self) -> Option<&std::path::Path> {
        match self {
            #[cfg(filesystem)]
            ResolvedExporterOptions::FileSystem(options) => Some(&options.root),
            #[cfg(feature = "collector")]
            ResolvedExporterOptions::Collector { .. } => None,
            #[cfg(feature = "callback")]
            ResolvedExporterOptions::Callback(_) => None,
        }
    }
}

/// Serialization format for the filesystem exporter and importer.
#[cfg(filesystem)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemFormat {
    #[cfg(feature = "ndjson")]
    Ndjson,
    #[cfg(feature = "msgpack")]
    Msgpack,
    #[cfg(feature = "postcard")]
    Postcard,
}

#[cfg(filesystem)]
impl TryFrom<&str> for FileSystemFormat {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value.to_ascii_lowercase().as_str() {
            #[cfg(feature = "ndjson")]
            "ndjson" => Self::Ndjson,
            #[cfg(feature = "msgpack")]
            "msgpack" => Self::Msgpack,
            #[cfg(feature = "postcard")]
            "postcard" => Self::Postcard,
            _ => return Err(format!("invalid filesystem format '{value}'")),
        })
    }
}

#[cfg(filesystem)]
impl FileSystemFormat {
    /// Detect the format of a context directory from the first recognized
    /// `*.<ext>` event stream in any of its per-entity subdirectories. Returns
    /// `None` if no readable stream with a known extension is present.
    pub fn detect(context_dir: &std::path::Path) -> Option<Self> {
        for entry in std::fs::read_dir(context_dir).ok()?.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let Ok(files) = std::fs::read_dir(entry.path()) else {
                continue;
            };
            for file in files.flatten() {
                if let Some(format) = std::path::Path::new(&file.file_name())
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .and_then(|ext| Self::try_from(ext).ok())
                {
                    return Some(format);
                }
            }
        }
        None
    }
}

/// Options for exporting events to the filesystem in the given `format`, under
/// the directory `root`, together with a `model.qmi` provenance sidecar.
#[cfg(filesystem)]
#[derive(Debug, Clone)]
pub struct FileSystemExporterOptions {
    pub format: FileSystemFormat,
    pub root: PathBuf,
}

impl ExporterOptions {
    /// Bind these options to the context `id`: scope a filesystem `root` to the
    /// context directory, or set the collector's source context id.
    #[cfg_attr(not(any(filesystem, feature = "collector")), allow(unused_variables))]
    pub fn resolve(self, id: Uuid) -> ResolvedExporterOptions {
        match self {
            #[cfg(filesystem)]
            ExporterOptions::FileSystem(mut options) => {
                options.root = options.root.join(id.to_string());
                ResolvedExporterOptions::FileSystem(options)
            }
            #[cfg(feature = "collector")]
            ExporterOptions::Collector(options) => ResolvedExporterOptions::Collector {
                address: options.address,
                source_context_id: id,
            },
            #[cfg(feature = "callback")]
            ExporterOptions::Callback(callback) => ResolvedExporterOptions::Callback(callback),
        }
    }
}

/// Selects an importer and its options.
#[cfg(filesystem)]
#[derive(Debug, Clone)]
pub enum ImporterOptions {
    FileSystem(FileSystemImporterOptions),
}

/// Options for importing events from the filesystem in the given `format`.
/// `path` is either a directory containing the event file (located by the
/// format's extension) or a direct file path.
#[cfg(filesystem)]
#[derive(Debug, Clone)]
pub struct FileSystemImporterOptions {
    pub format: FileSystemFormat,
    pub path: PathBuf,
}

/// Construct an importer from [`ImporterOptions`].
#[cfg(filesystem)]
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

/// Construct an exporter from [`ResolvedExporterOptions`].
pub async fn create_exporter<T>(
    kind: ResolvedExporterOptions,
) -> ExporterResult<Box<dyn Exporter<T>>>
where
    T: Serialize + Send + EntityEvent + 'static,
{
    match kind {
        #[cfg(filesystem)]
        ResolvedExporterOptions::FileSystem(FileSystemExporterOptions { format, root }) => {
            match format {
                #[cfg(feature = "ndjson")]
                FileSystemFormat::Ndjson => Ok(Box::new(
                    quent_exporter_ndjson::NdjsonExporter::try_new::<T>(
                        quent_exporter_ndjson::NdjsonExporterOptions { dir: root },
                    )
                    .await?,
                ) as Box<dyn Exporter<T>>),
                #[cfg(feature = "msgpack")]
                FileSystemFormat::Msgpack => Ok(Box::new(
                    quent_exporter_msgpack::MsgpackExporter::try_new::<T>(
                        quent_exporter_msgpack::MsgpackExporterOptions { dir: root },
                    )
                    .await?,
                ) as Box<dyn Exporter<T>>),
                #[cfg(feature = "postcard")]
                FileSystemFormat::Postcard => Ok(Box::new(
                    quent_exporter_postcard::PostcardExporter::try_new::<T>(
                        quent_exporter_postcard::PostcardExporterOptions { dir: root },
                    )
                    .await?,
                ) as Box<dyn Exporter<T>>),
            }
        }
        #[cfg(feature = "collector")]
        ResolvedExporterOptions::Collector {
            address,
            source_context_id,
        } => Ok(Box::new(
            quent_exporter_collector::CollectorExporter::<T>::try_new(address, source_context_id)
                .await
                .map_err(ExporterError::Other)?,
        ) as Box<dyn Exporter<T>>),
        #[cfg(feature = "callback")]
        ResolvedExporterOptions::Callback(callback) => {
            Ok(Box::new(CallbackExporter::new(callback)) as Box<dyn Exporter<T>>)
        }
    }
}
