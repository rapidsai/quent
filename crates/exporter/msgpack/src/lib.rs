// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter dumping events as length-prefixed MessagePack records into a file.
//!
//! File format: sequence of length-prefixed records.
//! Each record: `[4 bytes: payload length as u32 BE][payload: msgpack-encoded Event<T>]`
use std::{
    io::{BufReader, Read},
    marker::PhantomData,
    path::PathBuf,
};

use quent_build_info::ArtifactInfo;
use quent_events::Event;
use quent_exporter_types::{
    ARTIFACT_MAGIC, Exporter, ExporterError, ExporterResult, FORMAT_MSGPACK, Importer,
    ImporterResult, Preamble, read_preamble,
};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
    sync::Mutex,
};
use tracing::{debug, error};
use uuid::Uuid;

/// Options for the MessagePack exporter.
///
/// Writes events in MessagePack binary format. Compact and fast to
/// serialize/deserialize. Produces one file per instrumentation context
/// in `output_dir`.
#[derive(Debug, Clone)]
pub struct MsgpackExporterOptions {
    pub output_dir: PathBuf,
}

#[derive(Debug)]
pub struct MsgpackExporter {
    writer: Mutex<BufWriter<File>>,
}

impl MsgpackExporter {
    pub async fn try_new(
        application_id: Uuid,
        options: MsgpackExporterOptions,
        info: ArtifactInfo,
    ) -> ExporterResult<Self> {
        tokio::fs::create_dir_all(&options.output_dir).await?;
        let path = options
            .output_dir
            .join(format!("{}.msgpack", application_id));
        debug!("exporting to \"{}\"", path.display());
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        // Write the magic + header record only into a freshly created file.
        let is_new = file.metadata().await?.len() == 0;
        let mut writer = BufWriter::new(file);
        if is_new {
            let payload =
                rmp_serde::to_vec(&info).map_err(|e| ExporterError::Serde(format!("{e:?}")))?;
            writer.write_all(ARTIFACT_MAGIC).await?;
            writer.write_all(&[FORMAT_MSGPACK]).await?;
            writer
                .write_all(&(payload.len() as u32).to_be_bytes())
                .await?;
            writer.write_all(&payload).await?;
        }

        Ok(Self {
            writer: Mutex::new(writer),
        })
    }
}

#[async_trait::async_trait]
impl<T> Exporter<T> for MsgpackExporter
where
    T: Serialize + Send + 'static,
{
    async fn push(&self, event: Event<T>) -> ExporterResult<()> {
        let payload =
            rmp_serde::to_vec(&event).map_err(|e| ExporterError::Serde(format!("{e:?}")))?;
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
                let err = format!("unable to flush msgpack exporter: {e}");
                error!("{err}");
                Err(ExporterError::Flush(err))
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Options for the MessagePack importer. Reads events from the file at `path`.
pub struct MsgpackImporterOptions {
    pub path: PathBuf,
}

pub struct MsgpackImporter<T> {
    reader: BufReader<std::fs::File>,
    /// The first event record's length prefix, when the file had no header
    /// (legacy). Consumed by the first `next()`.
    peeked: Option<[u8; 4]>,
    _phantom: PhantomData<T>,
}

impl<T> MsgpackImporter<T> {
    pub fn try_new(options: &MsgpackImporterOptions) -> ImporterResult<Self> {
        let file = std::fs::File::open(&options.path)?;
        let mut reader = BufReader::new(file);
        let peeked = match read_preamble(&mut reader)? {
            Preamble::Legacy(buf) => Some(buf),
            Preamble::Header(_) | Preamble::Empty => None,
        };
        Ok(Self {
            reader,
            peeked,
            _phantom: Default::default(),
        })
    }
}

/// Read just the [`ArtifactInfo`] header from a msgpack artifact, if present.
pub fn read_header<R: Read>(mut reader: R) -> ImporterResult<Option<ArtifactInfo>> {
    Ok(match read_preamble(&mut reader)? {
        Preamble::Header(payload) => rmp_serde::from_slice::<ArtifactInfo>(&payload).ok(),
        Preamble::Legacy(_) | Preamble::Empty => None,
    })
}

impl<T> Importer<T> for MsgpackImporter<T> where T: for<'de> Deserialize<'de> {}

impl<T> Iterator for MsgpackImporter<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Item = Event<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let len_buf = match self.peeked.take() {
            Some(buf) => buf,
            None => {
                let mut buf = [0u8; 4];
                match self.reader.read_exact(&mut buf) {
                    Ok(()) => buf,
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return None,
                    Err(e) => {
                        error!("failed to read msgpack length: {e}");
                        return None;
                    }
                }
            }
        };
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        if let Err(e) = self.reader.read_exact(&mut payload) {
            error!("failed to read msgpack payload: {e}");
            return None;
        }
        match rmp_serde::from_slice::<Event<T>>(&payload) {
            Ok(event) => Some(event),
            Err(e) => {
                error!("failed to deserialize msgpack event: {e}");
                None
            }
        }
    }
}
