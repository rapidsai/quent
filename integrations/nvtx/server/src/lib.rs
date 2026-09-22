// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX HTTP views backed by the application's shared analyzer.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use moka::sync::Cache as SyncCache;
use nvtx_analyzer::NvtxModel;
use nvtx_ui::{NvtxCatalog, NvtxViewportRequest, NvtxViewportResponse};
use quent_analyzer::service::BlockingTasks;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use quent_query_engine_server::analyzer_cache::AnalyzerCache;
use uuid::Uuid;

const MAX_BODY_BYTES: usize = 64 * 1024;
const MAX_DOMAIN_FILTERS: usize = 256;
const MAX_CATEGORY_FILTERS: usize = 4096;
const MAX_CATALOG_ORIGINS: u64 = 32;
const MAX_CONCURRENT_MODEL_TASKS: usize = 4;

struct AnalyzerNvtxState<A, S>
where
    A: UiAnalyzer,
{
    analyzers: AnalyzerCache<A>,
    selector: Arc<S>,
    catalogs: SyncCache<(Uuid, u64), Arc<NvtxCatalog>>,
    tasks: BlockingTasks,
}

impl<A, S> Clone for AnalyzerNvtxState<A, S>
where
    A: UiAnalyzer,
{
    fn clone(&self) -> Self {
        Self {
            analyzers: self.analyzers.clone(),
            selector: Arc::clone(&self.selector),
            catalogs: self.catalogs.clone(),
            tasks: self.tasks.clone(),
        }
    }
}

impl<A, S> AnalyzerNvtxState<A, S>
where
    A: UiAnalyzer + Send + Sync + 'static,
    S: for<'a> Fn(&'a A, Uuid) -> Option<&'a NvtxModel> + Send + Sync + 'static,
{
    fn new(analyzers: AnalyzerCache<A>, selector: S) -> Self {
        Self {
            analyzers,
            selector: Arc::new(selector),
            catalogs: SyncCache::builder()
                .max_capacity(128 * MAX_CATALOG_ORIGINS)
                .time_to_idle(Duration::from_hours(24))
                .build(),
            tasks: BlockingTasks::new(
                NonZeroUsize::new(MAX_CONCURRENT_MODEL_TASKS).expect("nonzero task limit"),
            ),
        }
    }

    async fn analyzer(&self, context_id: Uuid) -> Result<Arc<A>, NvtxServerError> {
        self.analyzers
            .get_for_context(context_id)
            .await
            .map_err(|error| NvtxServerError::Internal(error.to_string()))?
            .ok_or(NvtxServerError::NotFound)
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct NvtxTimeOrigin {
    query_start: u64,
}

#[derive(Debug, Clone)]
enum NvtxServerError {
    BadRequest(String),
    NotFound,
    Internal(String),
}

impl From<quent_analyzer::service::TaskError> for NvtxServerError {
    fn from(error: quent_analyzer::service::TaskError) -> Self {
        Self::Internal(error.to_string())
    }
}

impl IntoResponse for NvtxServerError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            Self::NotFound => (StatusCode::NOT_FOUND, "NVTX stream not found").into_response(),
            Self::Internal(error) => {
                tracing::error!(%error, "NVTX context load failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "NVTX data could not be loaded",
                )
                    .into_response()
            }
        }
    }
}

/// Return stable metadata from an application analyzer relative to one query origin.
async fn analyzer_catalog<A, S>(
    State(state): State<AnalyzerNvtxState<A, S>>,
    AxumPath(context_id): AxumPath<Uuid>,
    Query(origin): Query<NvtxTimeOrigin>,
) -> Result<Json<NvtxCatalog>, NvtxServerError>
where
    A: UiAnalyzer + Send + Sync + 'static,
    S: for<'a> Fn(&'a A, Uuid) -> Option<&'a NvtxModel> + Send + Sync + 'static,
{
    let analyzer = state.analyzer(context_id).await?;
    let selector = Arc::clone(&state.selector);
    let catalogs = state.catalogs.clone();
    state
        .tasks
        .run(move || {
            let model = selector(&analyzer, context_id).ok_or(NvtxServerError::NotFound)?;
            let catalog = catalogs.get_with((context_id, origin.query_start), || {
                Arc::new(NvtxCatalog::from_model(model, origin.query_start))
            });
            Ok(Json((*catalog).clone()))
        })
        .await?
}

/// Build lanes and statistics from an application analyzer for one viewport.
async fn analyzer_viewport<A, S>(
    State(state): State<AnalyzerNvtxState<A, S>>,
    AxumPath(context_id): AxumPath<Uuid>,
    Query(origin): Query<NvtxTimeOrigin>,
    Json(request): Json<NvtxViewportRequest>,
) -> Result<Json<NvtxViewportResponse>, NvtxServerError>
where
    A: UiAnalyzer + Send + Sync + 'static,
    S: for<'a> Fn(&'a A, Uuid) -> Option<&'a NvtxModel> + Send + Sync + 'static,
{
    validate_filter_count(&request)?;
    let analyzer = state.analyzer(context_id).await?;
    let selector = Arc::clone(&state.selector);
    let catalogs = state.catalogs.clone();
    state
        .tasks
        .run(move || {
            let model = selector(&analyzer, context_id).ok_or(NvtxServerError::NotFound)?;
            let catalog = catalogs.get_with((context_id, origin.query_start), || {
                Arc::new(NvtxCatalog::from_model(model, origin.query_start))
            });
            NvtxViewportResponse::from_model_with_catalog(model, &catalog, request)
                .map(Json)
                .map_err(|error| NvtxServerError::BadRequest(error.to_string()))
        })
        .await?
}

/// Reject requests whose selector lists could cause disproportionate work.
fn validate_filter_count(request: &NvtxViewportRequest) -> Result<(), NvtxServerError> {
    if request.selections.len() > MAX_DOMAIN_FILTERS {
        return Err(NvtxServerError::BadRequest(
            "too many domain filters".to_owned(),
        ));
    }
    let category_count = request
        .selections
        .iter()
        .try_fold(0_usize, |total, selection| {
            total.checked_add(selection.category_ids.len())
        })
        .ok_or_else(|| NvtxServerError::BadRequest("too many category filters".to_owned()))?;
    if category_count > MAX_CATEGORY_FILTERS {
        return Err(NvtxServerError::BadRequest(
            "too many category filters".to_owned(),
        ));
    }
    Ok(())
}

/// Context-keyed NVTX API routes backed by an application's shared analyzers.
///
/// `selector` locates the NVTX model for the requested runtime context within
/// the application analyzer returned by `analyzers`. The analyzer remains the
/// sole owner of reconstruction; this router only caches UI catalogs by
/// `(context_id, query_start)` and derives viewport responses from the borrowed
/// model.
pub fn routes_from_analyzers<A, S>(analyzers: AnalyzerCache<A>, selector: S) -> Router
where
    A: UiAnalyzer + Send + Sync + 'static,
    S: for<'a> Fn(&'a A, Uuid) -> Option<&'a NvtxModel> + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/api/nvtx/contexts/{context_id}/catalog",
            get(analyzer_catalog::<A, S>),
        )
        .route(
            "/api/nvtx/contexts/{context_id}/viewport",
            post(analyzer_viewport::<A, S>),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(AnalyzerNvtxState::new(analyzers, selector))
}
