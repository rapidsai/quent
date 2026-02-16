use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use quent_exporter_msgpack::MsgpackImporter;
use quent_exporter_ndjson::NdjsonImporter;
use quent_exporter_postcard::PostcardImporter;
use quent_simulator_analyzer::Analyzer;
use tracing::info_span;
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
                let _span = info_span!("load_engine", %engine_id).entered();
                let postcard_path = format!("data/{engine_id}.postcard");
                let msgpack_path = format!("data/{engine_id}.msgpack");
                let ndjson_path = format!("data/{engine_id}.ndjson");
                if std::path::Path::new(&postcard_path).exists() {
                    let importer = PostcardImporter::try_new(&postcard_path)?;
                    Ok(Analyzer::try_new(engine_id, importer).map(Arc::new)?)
                } else if std::path::Path::new(&msgpack_path).exists() {
                    let importer = MsgpackImporter::try_new(&msgpack_path)?;
                    Ok(Analyzer::try_new(engine_id, importer).map(Arc::new)?)
                } else {
                    let importer = NdjsonImporter::try_new(&ndjson_path)?;
                    Ok(Analyzer::try_new(engine_id, importer).map(Arc::new)?)
                }
            })
            .await
            .map(|v| v.into_value())
            .map_err(|e: Arc<ServerError>| ServerError::Cache(format!("{e:?}")))
    }
}
