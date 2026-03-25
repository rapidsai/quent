// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter dumping events as newline-delimited JSON objects into a file.
use std::{
    io::{BufRead, BufReader},
    marker::PhantomData,
    path::PathBuf,
};

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

#[derive(Debug, Clone)]
pub struct NdjsonExporterOptions {
    pub output_dir: PathBuf,
}

#[derive(Debug)]
pub struct NdjsonExporter {
    writer: Mutex<BufWriter<File>>,
}

impl NdjsonExporter {
    pub async fn try_new(
        application_id: Uuid,
        options: NdjsonExporterOptions,
    ) -> ExporterResult<Self> {
        let path = options
            .output_dir
            .join(format!("{}.ndjson", application_id));
        debug!("exporting to \"{}\"", path.display());
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
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
pub struct NdjsonImporterOptions {
    pub path: PathBuf,
}

pub struct NdjsonImporter<T> {
    reader: BufReader<std::fs::File>,
    _phantom: PhantomData<T>,
}

impl<T> NdjsonImporter<T> {
    pub fn try_new(options: &NdjsonImporterOptions) -> ImporterResult<Self> {
        let file = std::fs::File::open(&options.path)?;
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
