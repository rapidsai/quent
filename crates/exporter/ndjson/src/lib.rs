// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter dumping events as newline-delimited JSON objects into a file.
use std::{
    io::{BufRead, BufReader},
    marker::PhantomData,
    path::PathBuf,
};

use quent_events::{EntityEvent, Event};
use quent_exporter_types::{Exporter, ExporterError, ExporterResult, Importer, ImporterResult};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
};
use tracing::{debug, error};
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

    async fn shutdown(&mut self) -> ExporterResult<()> {
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
    _phantom: PhantomData<T>,
}

impl<T> NdjsonImporter<T> {
    pub fn try_new(options: &NdjsonImporterOptions) -> ImporterResult<Self> {
        let path = quent_exporter_types::resolve_import_path(&options.path, "ndjson")?;
        let file = std::fs::File::open(&path)?;
        Ok(Self {
            reader: BufReader::new(file),
            _phantom: Default::default(),
        })
    }
}

impl<T> Importer<T> for NdjsonImporter<T> where T: for<'de> Deserialize<'de> {}

impl<T> Iterator for NdjsonImporter<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Item = Event<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => {
                let trimmed = line.trim_end();
                match serde_json::from_str::<Event<T>>(trimmed) {
                    Ok(event) => Some(event),
                    Err(e) => {
                        error!("failed to parse ndjson line: {e}");
                        None
                    }
                }
            }
            Err(e) => {
                error!("failed to read ndjson: {e}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestEvent;
    impl EntityEvent for TestEvent {
        const NAME: &'static str = "TestEvent";
    }

    #[tokio::test]
    async fn push_after_shutdown_errors() {
        let dir = tempfile::tempdir().unwrap();
        let mut exporter = NdjsonExporter::try_new::<TestEvent>(NdjsonExporterOptions {
            dir: dir.path().to_path_buf(),
        })
        .await
        .unwrap();

        exporter
            .push(Event::new_now(Uuid::now_v7(), TestEvent))
            .await
            .unwrap();
        Exporter::<TestEvent>::shutdown(&mut exporter)
            .await
            .unwrap();

        assert!(matches!(
            exporter
                .push(Event::new_now(Uuid::now_v7(), TestEvent))
                .await,
            Err(ExporterError::Shutdown)
        ));
        // A second shutdown is a no-op.
        Exporter::<TestEvent>::shutdown(&mut exporter)
            .await
            .unwrap();
    }
}
