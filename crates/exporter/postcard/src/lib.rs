// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter dumping events as length-prefixed postcard records into a file.
//!
//! File format: sequence of length-prefixed records.
//! Each record: `[4 bytes: payload length as u32 BE][payload: postcard-encoded Event<T>]`
use std::{io::BufReader, marker::PhantomData, path::PathBuf};

use quent_events::{EntityEvent, Event};
use quent_exporter_types::{Exporter, ExporterError, ExporterResult, Importer, ImporterResult};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
};
use tracing::{debug, error};
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

    async fn shutdown(&mut self) -> ExporterResult<()> {
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
    _phantom: PhantomData<T>,
}

impl<T> PostcardImporter<T> {
    pub fn try_new(options: &PostcardImporterOptions) -> ImporterResult<Self> {
        let path = quent_exporter_types::resolve_import_path(&options.path, "postcard")?;
        let file = std::fs::File::open(&path)?;
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
