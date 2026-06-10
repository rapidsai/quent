// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter dumping events as newline-delimited JSON objects into a file.
use std::{
    io::{BufRead, BufReader},
    marker::PhantomData,
    path::PathBuf,
};

use quent_build_info::ArtifactInfo;
use quent_events::Event;
use quent_exporter_types::{Exporter, ExporterError, ExporterResult, Importer, ImporterResult};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
    sync::Mutex,
};
use tracing::{debug, error};
use uuid::Uuid;

/// Options for the ndjson exporter.
///
/// Writes events as newline-delimited JSON (one JSON object per line per file).
/// Human-readable, useful for debugging and manual inspection. Produces one
/// file per instrumentation context in `output_dir`.
#[derive(Debug, Clone)]
pub struct NdjsonExporterOptions {
    pub output_dir: PathBuf,
}

#[derive(Debug)]
pub struct NdjsonExporter {
    writer: Mutex<BufWriter<File>>,
}

/// First line of an ndjson artifact: the provenance header. Distinguished from
/// `Event<T>` lines (which carry `id`/`timestamp`/`data`) by the
/// `__quent_header__` key.
#[derive(Debug, Serialize, Deserialize)]
struct NdjsonHeader {
    #[serde(rename = "__quent_header__")]
    version: u8,
    info: ArtifactInfo,
}

impl NdjsonExporter {
    pub async fn try_new(
        application_id: Uuid,
        options: NdjsonExporterOptions,
        info: ArtifactInfo,
    ) -> ExporterResult<Self> {
        tokio::fs::create_dir_all(&options.output_dir).await?;
        let path = options
            .output_dir
            .join(format!("{}.ndjson", application_id));
        debug!("exporting to \"{}\"", path.display());
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        // Write the header only into a freshly created (empty) file — never
        // append a second header when re-opening an existing artifact.
        let is_new = file.metadata().await?.len() == 0;
        let mut writer = BufWriter::new(file);
        if is_new {
            let header = NdjsonHeader { version: 1, info };
            let line = format!(
                "{}\n",
                serde_json::to_string(&header)
                    .map_err(|e| ExporterError::Serde(format!("{e:?}")))?
            );
            writer.write_all(line.as_bytes()).await?;
        }

        Ok(Self {
            writer: Mutex::new(writer),
        })
    }
}

#[async_trait::async_trait]
impl<T> Exporter<T> for NdjsonExporter
where
    T: Serialize + Send + 'static,
{
    async fn push(&self, event: Event<T>) -> ExporterResult<()> {
        let line = format!(
            "{}\n",
            serde_json::to_string(&event).map_err(|e| ExporterError::Serde(format!("{e:?}")))?
        );
        let mut lock = self.writer.lock().await;
        lock.write_all(line.as_bytes()).await?;
        Ok(())
    }

    async fn force_flush(&self) -> ExporterResult<()> {
        match self.writer.lock().await.flush().await {
            Ok(_) => Ok(()),
            Err(e) => {
                let err = format!("unable to flush ndjson exporter: {e}");
                error!("{err}");
                Err(ExporterError::Flush(err))
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Options for the ndjson importer. Reads events from the file at `path`.
pub struct NdjsonImporterOptions {
    pub path: PathBuf,
}

pub struct NdjsonImporter<T> {
    reader: BufReader<std::fs::File>,
    /// A first event line read while skipping the header in [`Self::try_new`],
    /// replayed by the first `next()` (legacy headerless files).
    peeked_line: Option<String>,
    _phantom: PhantomData<T>,
}

impl<T> NdjsonImporter<T> {
    pub fn try_new(options: &NdjsonImporterOptions) -> ImporterResult<Self> {
        let file = std::fs::File::open(&options.path)?;
        let mut reader = BufReader::new(file);
        let peeked_line = skip_header(&mut reader)?;
        Ok(Self {
            reader,
            peeked_line,
            _phantom: Default::default(),
        })
    }
}

/// Returns `true` if `line` is a provenance header rather than an `Event<T>`.
fn is_header_line(line: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(line.trim())
        .ok()
        .and_then(|value| {
            value
                .as_object()
                .map(|object| object.contains_key("__quent_header__"))
        })
        .unwrap_or(false)
}

/// Consume the header line if present. Returns `Some(line)` when the first line
/// was actually an event (legacy headerless file), to be replayed by `next()`.
fn skip_header<R: BufRead>(reader: &mut R) -> ImporterResult<Option<String>> {
    let mut first = String::new();
    if reader.read_line(&mut first)? == 0 {
        return Ok(None);
    }
    if is_header_line(&first) {
        Ok(None)
    } else {
        Ok(Some(first))
    }
}

/// Read just the [`ArtifactInfo`] header from an ndjson artifact, if present.
pub fn read_header<R: BufRead>(mut reader: R) -> ImporterResult<Option<ArtifactInfo>> {
    let mut first = String::new();
    if reader.read_line(&mut first)? == 0 {
        return Ok(None);
    }
    Ok(serde_json::from_str::<NdjsonHeader>(first.trim())
        .ok()
        .map(|header| header.info))
}

impl<T> Importer<T> for NdjsonImporter<T> where T: for<'de> Deserialize<'de> {}

impl<T> Iterator for NdjsonImporter<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Item = Event<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = match self.peeked_line.take() {
            Some(line) => line,
            None => {
                let mut line = String::new();
                match self.reader.read_line(&mut line) {
                    Ok(0) => return None,
                    Ok(_) => line,
                    Err(e) => {
                        error!("failed to read ndjson: {e}");
                        return None;
                    }
                }
            }
        };
        let trimmed = line.trim_end();
        match serde_json::from_str::<Event<T>>(trimmed) {
            Ok(event) => Some(event),
            Err(e) => {
                error!("failed to parse ndjson line: {e}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_build_info::{ArtifactInfo, ModelInfo};
    use uuid::Uuid;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("quent-ndjson-{tag}-{}", Uuid::now_v7()))
    }

    #[tokio::test]
    async fn header_written_once_then_skipped_on_import() {
        let dir = temp_dir("hdr");
        let id = Uuid::now_v7();
        let info = ArtifactInfo::new(ModelInfo::unknown());
        let opts = NdjsonExporterOptions {
            output_dir: dir.clone(),
        };

        let exporter = NdjsonExporter::try_new(id, opts.clone(), info.clone())
            .await
            .unwrap();
        Exporter::<String>::push(&exporter, Event::new(id, 1, "a".to_string()))
            .await
            .unwrap();
        Exporter::<String>::force_flush(&exporter).await.unwrap();
        drop(exporter);

        // Re-open the same artifact (append mode): must not write a second header.
        let exporter = NdjsonExporter::try_new(id, opts, info).await.unwrap();
        Exporter::<String>::push(&exporter, Event::new(id, 2, "b".to_string()))
            .await
            .unwrap();
        Exporter::<String>::force_flush(&exporter).await.unwrap();
        drop(exporter);

        let path = dir.join(format!("{id}.ndjson"));
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents.matches("__quent_header__").count(), 1);

        let header =
            read_header(std::io::BufReader::new(std::fs::File::open(&path).unwrap())).unwrap();
        assert_eq!(header.unwrap().model.name, "unknown");

        let events: Vec<_> = NdjsonImporter::<String>::try_new(&NdjsonImporterOptions { path })
            .unwrap()
            .collect();
        assert_eq!(
            events.iter().map(|e| e.data.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn legacy_headerless_file_imports() {
        let dir = temp_dir("legacy");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("legacy.ndjson");
        let event = Event::new(Uuid::nil(), 7, "x".to_string());
        std::fs::write(
            &path,
            format!("{}\n", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();

        let events: Vec<_> =
            NdjsonImporter::<String>::try_new(&NdjsonImporterOptions { path: path.clone() })
                .unwrap()
                .collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "x");

        std::fs::remove_dir_all(&dir).ok();
    }
}
