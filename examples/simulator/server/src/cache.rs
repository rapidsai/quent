use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use quent_exporter_ndjson::NdjsonImporter;
use quent_simulator_analyzer::Analyzer;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};

// Clone is cheap, see:
// https://docs.rs/moka/latest/moka/future/struct.Cache.html#sharing-a-cache-across-asynchronous-tasks
#[derive(Clone)]
pub struct AnalyzerCache {
    analyzers: Cache<Uuid, Arc<Analyzer>>,
}

impl AnalyzerCache {
    pub(crate) fn new() -> Self {
        Self {
            analyzers: Cache::builder()
                .max_capacity(32)
                .time_to_idle(Duration::from_hours(24))
                .build(),
        }
    }

    pub(crate) async fn get(&self, engine_id: Uuid) -> ServerResult<Arc<Analyzer>> {
        self.analyzers
            .entry(engine_id)
            .or_try_insert_with(async {
                let importer = NdjsonImporter::try_new(format!("data/{engine_id}.ndjson"))?;
                Ok(Analyzer::try_new(engine_id, importer).map(Arc::new)?)
            })
            .await
            .map(|v| v.into_value())
            .map_err(|e: Arc<ServerError>| ServerError::Cache(format!("{e:?}")))
    }
}
