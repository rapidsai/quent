//! Exporter dumping events as newline-delimited JSON objects into a file.
use std::{
    io::{BufRead, BufReader},
    path::Path,
};

use quent_events::{Event, EventData};
use quent_exporter::Exporter;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
    sync::Mutex,
};
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug)]
pub struct NdjsonExporter {
    writer: Mutex<BufWriter<File>>,
}

impl NdjsonExporter {
    pub async fn try_new(engine_id: Uuid) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO(johanpel): path config
        let path = format!("data/{}.ndjson", engine_id);

        debug!("exporting to \"{path}\"");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        let writer = Mutex::new(BufWriter::new(file));

        Ok(Self { writer })
    }
}

#[async_trait::async_trait]
impl Exporter for NdjsonExporter {
    async fn push(
        &self,
        event: quent_events::Event<quent_events::EventData>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let line = format!("{}\n", serde_json::to_string(&event)?);
        let mut lock = self.writer.lock().await;
        lock.write_all(line.as_bytes()).await?;
        lock.flush().await?;
        Ok(())
    }

    async fn force_flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self.writer.lock().await.flush().await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("unable to flush ndjson exporter: {e}");
                Err(Box::new(e))
            }
        }
    }
}

pub struct NdjsonImporter {
    reader: BufReader<std::fs::File>,
}

impl NdjsonImporter {
    pub fn try_new(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }
}

impl Iterator for NdjsonImporter {
    type Item = Event<EventData>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => {
                let trimmed = line.trim_end();
                match serde_json::from_str::<Event<EventData>>(trimmed) {
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
