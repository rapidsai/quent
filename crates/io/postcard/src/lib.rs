// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter dumping events as length-prefixed postcard records into a file.
//!
//! File format: sequence of length-prefixed records.
//! Each record: `[4 bytes: payload length as u32 BE][payload: postcard-encoded Event<T>]`
use std::{
    io::{BufReader, Read},
    marker::PhantomData,
    path::PathBuf,
};

use quent_events::{EntityEvent, Event};
use quent_io_types::{
    Exporter, ExporterError, ExporterResult, Importer, ImporterError, ImporterResult,
    MAX_FRAME_SIZE_BYTES,
};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
};
use tracing::{debug, warn};
use uuid::Uuid;

/// File extension for Postcard event files.
const EXTENSION: &str = "postcard";

/// Options for the Postcard exporter.
///
/// A compact row-oriented binary format, which is not self-describing.
///
/// Writes events in Postcard format under `dir`, in a per-entity subdirectory
/// holding a UUIDv7-named `.postcard` file.
#[derive(Debug, Clone)]
pub struct PostcardExporterOptions {
    pub dir: PathBuf,
}

#[derive(Debug)]
pub struct PostcardExporter {
    /// `None` once [`shutdown`](Exporter::shutdown) has flushed and released it.
    writer: Option<BufWriter<File>>,
    /// Framing buffer reused across [`drain_events`](Exporter::drain_events).
    batch: Vec<u8>,
}

impl PostcardExporter {
    pub async fn try_new<T: EntityEvent>(options: PostcardExporterOptions) -> ExporterResult<Self> {
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
            batch: Vec::new(),
        })
    }
}

#[async_trait::async_trait]
impl<T> Exporter<T> for PostcardExporter
where
    T: Serialize + Send + EntityEvent + 'static,
{
    async fn push(&mut self, event: Event<T>) -> ExporterResult<()> {
        let writer = self.writer.as_mut().ok_or(ExporterError::Shutdown)?;
        let payload = postcard::to_allocvec(&event).map_err(ExporterError::other)?;
        let len = (payload.len() as u32).to_be_bytes();
        writer.write_all(&len).await?;
        writer.write_all(&payload).await?;
        Ok(())
    }

    async fn drain_events(&mut self, events: &mut Vec<Event<T>>) -> ExporterResult<()> {
        let Self { writer, batch } = self;
        let Some(writer) = writer.as_mut() else {
            events.clear();
            return Err(ExporterError::Shutdown);
        };
        // Frame the whole batch into the reused buffer, then issue a single
        // write. A record that fails to serialize is logged and skipped so one
        // bad event does not drop the batch.
        batch.clear();
        for event in events.drain(..) {
            match postcard::to_allocvec(&event) {
                Ok(payload) => {
                    batch.extend_from_slice(&(payload.len() as u32).to_be_bytes());
                    batch.extend_from_slice(&payload);
                }
                Err(e) => warn!("unable to serialize event: {e}"),
            }
        }
        writer.write_all(batch).await?;
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

/// Options for the Postcard importer. `path` is either the directory containing
/// the event file (located by its `.postcard` extension) or the file itself.
#[derive(Debug, Clone)]
pub struct PostcardImporterOptions {
    pub path: PathBuf,
}

pub struct PostcardImporter<T> {
    reader: BufReader<std::fs::File>,
    terminated: bool,
    _phantom: PhantomData<T>,
}

impl<T> PostcardImporter<T> {
    pub fn try_new(options: &PostcardImporterOptions) -> ImporterResult<Self> {
        let path = quent_io_types::resolve_import_path(&options.path, "postcard")?;
        let file = std::fs::File::open(&path)?;
        Ok(Self {
            reader: BufReader::new(file),
            terminated: false,
            _phantom: Default::default(),
        })
    }
}

impl<T> Importer<T> for PostcardImporter<T> where T: for<'de> Deserialize<'de> {}

impl<T> Iterator for PostcardImporter<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Item = ImporterResult<Event<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminated {
            return None;
        }

        let mut len_buf = [0u8; 4];
        match self.reader.read(&mut len_buf[..1]) {
            Ok(0) => return None,
            Ok(_) => {}
            // The reader position after an I/O failure may not be a frame boundary.
            Err(error) => return self.fail(error.into()),
        }
        if let Err(error) = self.reader.read_exact(&mut len_buf[1..]) {
            // An incomplete length prefix does not identify the next frame boundary.
            return self.fail(error.into());
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_FRAME_SIZE_BYTES {
            // Consuming an unsupported payload could require unbounded I/O before resuming.
            return self.fail(ImporterError::other(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "frame size {len} exceeds the supported maximum of {MAX_FRAME_SIZE_BYTES} bytes"
                ),
            )));
        }
        let mut payload = Vec::new();
        if let Err(error) = payload.try_reserve_exact(len) {
            // Without a payload buffer, this importer cannot decode the current frame.
            return self.fail(ImporterError::other(error));
        }
        payload.resize(len, 0);
        if let Err(error) = self.reader.read_exact(&mut payload) {
            // An incomplete payload leaves the reader before the next frame boundary.
            return self.fail(error.into());
        }
        match postcard::from_bytes::<Event<T>>(&payload) {
            Ok(event) => Some(Ok(event)),
            Err(error) => Some(Err(ImporterError::other(error))),
        }
    }
}

impl<T> PostcardImporter<T> {
    fn fail(&mut self, error: quent_io_types::ImporterError) -> Option<ImporterResult<Event<T>>> {
        self.terminated = true;
        Some(Err(error))
    }
}
