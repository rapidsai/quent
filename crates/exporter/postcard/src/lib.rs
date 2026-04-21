// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter dumping events as length-prefixed postcard records into a file.
//!
//! File format: sequence of length-prefixed records.
//! Each record: `[4 bytes: payload length as u32 BE][payload: postcard-encoded Event<T>]`
use std::{io::BufReader, marker::PhantomData, path::PathBuf};

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

/// Options for the Postcard exporter.
///
/// Writes events in Postcard format (a compact, no_std-friendly binary
/// encoding). Produces one file per instrumentation context in `output_dir`.
#[derive(Debug, Clone)]
pub struct PostcardExporterOptions {
    pub output_dir: PathBuf,
}

#[derive(Debug)]
pub struct PostcardExporter {
    writer: Mutex<BufWriter<File>>,
}

impl PostcardExporter {
    pub async fn try_new(
        application_id: Uuid,
        options: PostcardExporterOptions,
    ) -> ExporterResult<Self> {
        tokio::fs::create_dir_all(&options.output_dir).await?;
        let path = options
            .output_dir
            .join(format!("{}.postcard", application_id));
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
impl<T> Exporter<T> for PostcardExporter
where
    T: Serialize + Send + 'static,
{
    async fn push(&self, event: Event<T>) -> ExporterResult<()> {
        let payload =
            postcard::to_allocvec(&event).map_err(|e| ExporterError::Serde(format!("{e:?}")))?;
        let len = (payload.len() as u32).to_be_bytes();
        let mut lock = self.writer.lock().await;
        lock.write_all(&len).await?;
        lock.write_all(&payload).await?;
        Ok(())
    }

    async fn force_flush(&self) -> ExporterResult<()> {
        match self.writer.lock().await.flush().await {
            Ok(_) => Ok(()),
            Err(e) => {
                let err = format!("unable to flush postcard exporter: {e}");
                error!("{err}");
                Err(ExporterError::Flush(err))
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Options for the Postcard importer. Reads events from the file at `path`.
pub struct PostcardImporterOptions {
    pub path: PathBuf,
}

pub struct PostcardImporter<T> {
    reader: BufReader<std::fs::File>,
    _phantom: PhantomData<T>,
}

impl<T> PostcardImporter<T> {
    pub fn try_new(options: &PostcardImporterOptions) -> ImporterResult<Self> {
        let file = std::fs::File::open(&options.path)?;
        Ok(Self {
            reader: BufReader::new(file),
            _phantom: Default::default(),
        })
    }
}

impl<T> Importer<T> for PostcardImporter<T> where T: for<'de> Deserialize<'de> {}

impl<T> Iterator for PostcardImporter<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Item = Event<T>;

    fn next(&mut self) -> Option<Self::Item> {
        use std::io::Read;
        let mut len_buf = [0u8; 4];
        match self.reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return None,
            Err(e) => {
                error!("failed to read postcard length: {e}");
                return None;
            }
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        if let Err(e) = self.reader.read_exact(&mut payload) {
            error!("failed to read postcard payload: {e}");
            return None;
        }
        match postcard::from_bytes::<Event<T>>(&payload) {
            Ok(event) => Some(event),
            Err(e) => {
                error!("failed to deserialize postcard event: {e}");
                None
            }
        }
    }
}
