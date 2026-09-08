// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter dumping events as newline-delimited JSON objects into a file.
use std::{
    io::{BufRead, BufReader},
    marker::PhantomData,
    path::PathBuf,
};

use quent_events::{EntityEvent, Event};
use quent_io_types::{
    Exporter, ExporterError, ExporterResult, Importer, ImporterError, ImporterResult,
};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
};
use tracing::{debug, warn};
use uuid::Uuid;

/// File extension for ndjson event files.
const EXTENSION: &str = "ndjson";

/// Options for the ndjson exporter.
///
/// A human-readable format useful for debugging and manual / LLM-based
/// inspection.
///
/// Writes events as newline-delimited JSON (one JSON object per line) under
/// `dir`, in a per-entity subdirectory holding a UUIDv7-named `.ndjson` file.
#[derive(Debug, Clone)]
pub struct NdjsonExporterOptions {
    pub dir: PathBuf,
}

#[derive(Debug)]
pub struct NdjsonExporter {
    /// `None` once [`shutdown`](Exporter::shutdown) has flushed and released it.
    writer: Option<BufWriter<File>>,
    /// Line buffer reused across [`drain_events`](Exporter::drain_events).
    batch: String,
}

impl NdjsonExporter {
    pub async fn try_new<T: EntityEvent>(options: NdjsonExporterOptions) -> ExporterResult<Self> {
        let dir = options.dir.join(T::NAME);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("{}.{EXTENSION}", Uuid::now_v7()));
        debug!("exporting to \"{}\"", path.display());
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        Ok(Self {
            writer: Some(BufWriter::new(file)),
            batch: String::new(),
        })
    }
}

#[async_trait::async_trait]
impl<T> Exporter<T> for NdjsonExporter
where
    T: Serialize + Send + EntityEvent + 'static,
{
    async fn push(&mut self, event: Event<T>) -> ExporterResult<()> {
        let writer = self.writer.as_mut().ok_or(ExporterError::Shutdown)?;
        let line = format!(
            "{}\n",
            serde_json::to_string(&event).map_err(ExporterError::other)?
        );
        writer.write_all(line.as_bytes()).await?;
        Ok(())
    }

    async fn drain_events(&mut self, events: &mut Vec<Event<T>>) -> ExporterResult<()> {
        let Self { writer, batch } = self;
        let Some(writer) = writer.as_mut() else {
            events.clear();
            return Err(ExporterError::Shutdown);
        };
        // Concatenate the whole batch into the reused buffer, then issue a
        // single write. A record that fails to serialize is logged and skipped
        // so one bad event does not drop the batch.
        batch.clear();
        for event in events.drain(..) {
            match serde_json::to_string(&event) {
                Ok(line) => {
                    batch.push_str(&line);
                    batch.push('\n');
                }
                Err(e) => warn!("unable to serialize event: {e}"),
            }
        }
        writer.write_all(batch.as_bytes()).await?;
        Ok(())
    }

    async fn shutdown(mut self: Box<Self>) -> ExporterResult<()> {
        let Some(mut writer) = self.writer.take() else {
            return Ok(());
        };
        writer.flush().await?;
        Ok(())
    }
}

/// Options for the ndjson importer. `path` is either the directory containing
/// the event file (located by its `.ndjson` extension) or the file itself.
#[derive(Debug, Clone)]
pub struct NdjsonImporterOptions {
    pub path: PathBuf,
}

pub struct NdjsonImporter<T> {
    reader: BufReader<std::fs::File>,
    terminated: bool,
    _phantom: PhantomData<T>,
}

impl<T> NdjsonImporter<T> {
    pub fn try_new(options: &NdjsonImporterOptions) -> ImporterResult<Self> {
        let path = quent_io_types::resolve_import_path(&options.path, "ndjson")?;
        let file = std::fs::File::open(&path)?;
        Ok(Self {
            reader: BufReader::new(file),
            terminated: false,
            _phantom: Default::default(),
        })
    }
}

impl<T> Importer<T> for NdjsonImporter<T> where T: for<'de> Deserialize<'de> {}

impl<T> Iterator for NdjsonImporter<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Item = ImporterResult<Event<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminated {
            return None;
        }

        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => match serde_json::from_str::<Event<T>>(line.trim_end()) {
                Ok(event) => Some(Ok(event)),
                Err(error) => Some(Err(ImporterError::other(error))),
            },
            Err(e) => {
                // The failed read may have consumed a partial line without its delimiter.
                self.terminated = true;
                Some(Err(e.into()))
            }
        }
    }
}
