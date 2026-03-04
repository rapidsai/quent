use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use quent_analyzer::AnalyzerResult;
use quent_events::Event;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use tracing::info_span;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};

pub type ImporterFn<A> = dyn Fn(Uuid) -> AnalyzerResult<Box<dyn Iterator<Item = Event<<A as UiAnalyzer>::Event>>>>
    + Send
    + Sync;

/// Cache for analyzer instances, keyed by engine ID.
pub struct AnalyzerCache<A>
where
    A: UiAnalyzer,
{
    analyzers: Cache<Uuid, Arc<A>>,
    importer: Arc<ImporterFn<A>>,
}

impl<A> Clone for AnalyzerCache<A>
where
    A: UiAnalyzer,
{
    fn clone(&self) -> Self {
        Self {
            analyzers: self.analyzers.clone(),
            importer: Arc::clone(&self.importer),
        }
    }
}

impl<A> AnalyzerCache<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    pub(crate) fn new(importer: Box<ImporterFn<A>>) -> Self {
        Self {
            analyzers: Cache::builder()
                .max_capacity(32)
                .time_to_idle(Duration::from_hours(24))
                .build(),
            importer: Arc::from(importer),
        }
    }

    pub(crate) async fn get(&self, engine_id: Uuid) -> ServerResult<Arc<A>> {
        let importer = Arc::clone(&self.importer);
        self.analyzers
            .entry(engine_id)
            .or_try_insert_with(async {
                tokio::task::spawn_blocking(move || -> ServerResult<Arc<A>> {
                    let _span = info_span!("load_engine", %engine_id).entered();
                    let events = importer(engine_id)?;
                    Ok(A::try_new(engine_id, events).map(Arc::new)?)
                })
                .await
                .map_err(|e| ServerError::Cache(format!("blocking task panicked: {e}")))?
            })
            .await
            .map(|v| v.into_value())
            .map_err(|e: Arc<ServerError>| ServerError::Cache(format!("{e:?}")))
    }
}
