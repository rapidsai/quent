// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use quent_events::EntityEvent;
use quent_io_types::{Exporter, ExporterProvider, ExporterResult};
use serde::Serialize;
use uuid::Uuid;

use crate::filesystem::Format;

/// Options for exporting events to the filesystem in the given `format`, under
/// `root/<context_id>`, together with a `model.qmi` provenance sidecar.
#[derive(Debug, Clone)]
pub struct Options {
    format: Format,
    root: PathBuf,
}

impl Options {
    pub fn new(format: Format, root: PathBuf) -> Self {
        Self { format, root }
    }

    /// The per-context output directory, `root/<context_id>`.
    pub(crate) fn dir(&self, context_id: Uuid) -> PathBuf {
        self.root.join(context_id.to_string())
    }
}

#[async_trait::async_trait]
impl<T> ExporterProvider<T> for Options
where
    T: Send + EntityEvent + 'static,
    T: Serialize,
{
    async fn create_exporter(&self, context_id: Uuid) -> ExporterResult<Box<dyn Exporter<T>>> {
        let dir = self.dir(context_id);
        match self.format {
            #[cfg(feature = "ndjson")]
            Format::Ndjson => Ok(Box::new(
                quent_io_ndjson::NdjsonExporter::try_new::<T>(
                    quent_io_ndjson::NdjsonExporterOptions { dir },
                )
                .await?,
            ) as Box<dyn Exporter<T>>),
            #[cfg(feature = "msgpack")]
            Format::Msgpack => Ok(Box::new(
                quent_io_msgpack::MsgpackExporter::try_new::<T>(
                    quent_io_msgpack::MsgpackExporterOptions { dir },
                )
                .await?,
            ) as Box<dyn Exporter<T>>),
            #[cfg(feature = "postcard")]
            Format::Postcard => Ok(Box::new(
                quent_io_postcard::PostcardExporter::try_new::<T>(
                    quent_io_postcard::PostcardExporterOptions { dir },
                )
                .await?,
            ) as Box<dyn Exporter<T>>),
        }
    }
}
