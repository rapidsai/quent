//! Exporter dumping events as newline-delimited JSON objects into a file.
use quent_exporter::Exporter;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
    sync::Mutex,
};
use tracing::{debug, error};
use uuid::Uuid;

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
