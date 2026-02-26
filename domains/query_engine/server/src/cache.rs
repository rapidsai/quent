use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use tracing::info_span;
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};

pub struct AnalyzerCache<A>
where
    A: UiAnalyzer,
{
    analyzers: Cache<Uuid, Arc<A>>,
}

impl<A> Clone for AnalyzerCache<A>
where
    A: UiAnalyzer,
{
    fn clone(&self) -> Self {
        Self {
            analyzers: self.analyzers.clone(),
        }
    }
}

impl<A> AnalyzerCache<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    pub(crate) fn new() -> Self {
        Self {
            analyzers: Cache::builder()
                .max_capacity(32)
                .time_to_idle(Duration::from_hours(24))
                .build(),
        }
    }

    pub(crate) async fn get(&self, engine_id: Uuid) -> ServerResult<Arc<A>> {
        self.analyzers
            .entry(engine_id)
            .or_try_insert_with(async {
                tokio::task::spawn_blocking(move || -> ServerResult<Arc<A>> {
                    let _span = info_span!("load_engine", %engine_id).entered();
                    Ok(A::try_new(engine_id).map(Arc::new)?)
                })
                .await
                .map_err(|e| ServerError::Cache(format!("blocking task panicked: {e}")))?
            })
            .await
            .map(|v| v.into_value())
            .map_err(|e: Arc<ServerError>| ServerError::Cache(format!("{e:?}")))
    }
}
