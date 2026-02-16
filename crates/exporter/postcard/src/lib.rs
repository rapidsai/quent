//! Exporter dumping events as length-prefixed postcard records into a file.
//!
//! File format: sequence of length-prefixed records.
//! Each record: [4 bytes: payload length as u32 BE][payload: postcard-encoded Event<T>]
use std::{io::BufReader, marker::PhantomData, path::Path};

use quent_events::Event;
use quent_exporter::{Exporter, ExporterError, ExporterResult, ImporterResult};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
    sync::Mutex,
};
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug)]
pub struct PostcardExporter {
    writer: Mutex<BufWriter<File>>,
}

impl PostcardExporter {
    pub async fn try_new(engine_id: Uuid) -> ExporterResult<Self> {
        let path = format!("data/{}.postcard", engine_id);
        debug!("exporting to \"{path}\"");
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

pub struct PostcardImporter<T> {
    reader: BufReader<std::fs::File>,
    _phantom: PhantomData<T>,
}

impl<T> PostcardImporter<T> {
    pub fn try_new(path: impl AsRef<Path>) -> ImporterResult<Self> {
        let file = std::fs::File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
            _phantom: Default::default(),
        })
    }
}

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
