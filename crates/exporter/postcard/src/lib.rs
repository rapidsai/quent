// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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

use quent_build_info::ArtifactInfo;
use quent_events::Event;
use quent_exporter_types::{
    ARTIFACT_MAGIC, Exporter, ExporterError, ExporterResult, FORMAT_POSTCARD, Importer,
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
        info: ArtifactInfo,
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

        // Write the magic + header record only into a freshly created file.
        let is_new = file.metadata().await?.len() == 0;
        let mut writer = BufWriter::new(file);
        if is_new {
            let payload =
                postcard::to_allocvec(&info).map_err(|e| ExporterError::Serde(format!("{e:?}")))?;
            writer.write_all(ARTIFACT_MAGIC).await?;
            writer.write_all(&[FORMAT_POSTCARD]).await?;
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
    /// The first event record's length prefix, when the file had no header
    /// (legacy). Consumed by the first `next()`.
    peeked: Option<[u8; 4]>,
    _phantom: PhantomData<T>,
}

impl<T> PostcardImporter<T> {
    pub fn try_new(options: &PostcardImporterOptions) -> ImporterResult<Self> {
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

/// Read just the [`ArtifactInfo`] header from a postcard artifact, if present.
pub fn read_header<R: Read>(mut reader: R) -> ImporterResult<Option<ArtifactInfo>> {
    Ok(match read_preamble(&mut reader)? {
        Preamble::Header(payload) => postcard::from_bytes::<ArtifactInfo>(&payload).ok(),
        Preamble::Legacy(_) | Preamble::Empty => None,
    })
}

impl<T> Importer<T> for PostcardImporter<T> where T: for<'de> Deserialize<'de> {}

impl<T> Iterator for PostcardImporter<T>
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
                        error!("failed to read postcard length: {e}");
                        return None;
                    }
                }
            }
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use quent_build_info::{ArtifactInfo, ModelInfo};
    use uuid::Uuid;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("quent-postcard-{tag}-{}", Uuid::now_v7()))
    }

    #[tokio::test]
    async fn header_written_once_then_skipped_on_import() {
        let dir = temp_dir("hdr");
        let id = Uuid::now_v7();
        let info = ArtifactInfo::new(ModelInfo::unknown());
        let opts = PostcardExporterOptions {
            output_dir: dir.clone(),
        };

        let exporter = PostcardExporter::try_new(id, opts.clone(), info.clone())
            .await
            .unwrap();
        Exporter::<String>::push(&exporter, Event::new(id, 1, "a".to_string()))
            .await
            .unwrap();
        Exporter::<String>::force_flush(&exporter).await.unwrap();
        drop(exporter);

        // Re-open the same artifact (append mode): must not write a second header.
        let exporter = PostcardExporter::try_new(id, opts, info).await.unwrap();
        Exporter::<String>::push(&exporter, Event::new(id, 2, "b".to_string()))
            .await
            .unwrap();
        Exporter::<String>::force_flush(&exporter).await.unwrap();
        drop(exporter);

        let path = dir.join(format!("{id}.postcard"));
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], &ARTIFACT_MAGIC[..]);
        assert_eq!(bytes[4], FORMAT_POSTCARD);

        let header = read_header(std::io::Cursor::new(bytes.as_slice())).unwrap();
        assert_eq!(header.unwrap().model.name, "unknown");

        let events: Vec<_> = PostcardImporter::<String>::try_new(&PostcardImporterOptions { path })
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
        let path = dir.join("legacy.postcard");
        // A raw length-prefixed record with no magic, as written before this change.
        let event = Event::new(Uuid::nil(), 7, "x".to_string());
        let payload = postcard::to_allocvec(&event).unwrap();
        let mut bytes = (payload.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(&payload);
        std::fs::write(&path, bytes).unwrap();

        let events: Vec<_> = PostcardImporter::<String>::try_new(&PostcardImporterOptions { path })
            .unwrap()
            .collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "x");

        std::fs::remove_dir_all(&dir).ok();
    }
}
