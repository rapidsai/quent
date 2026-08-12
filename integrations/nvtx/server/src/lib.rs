// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Context-keyed NVTX reconstruction cache and Axum routes.

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use moka::{future::Cache as AsyncCache, sync::Cache as SyncCache};
use nvtx_analyzer::{NvtxModel, NvtxModelBuilder};
use nvtx_bridge::NvtxEventEntity;
use nvtx_ui::{NvtxCatalog, NvtxViewportRequest, NvtxViewportResponse};
use quent_events::{EntityEvent, Event};
use quent_io::filesystem::{self, Format};
use quent_io::{ImporterOptions, ImporterProvider};
use uuid::Uuid;

const MAX_BODY_BYTES: usize = 64 * 1024;
const MAX_DOMAIN_FILTERS: usize = 256;
const MAX_CATEGORY_FILTERS: usize = 4096;
const MAX_CATALOG_ORIGINS: u64 = 32;

/// A redacted importer failure. Details are logged server-side and are never
/// copied into an HTTP response.
#[derive(Debug, Clone)]
pub struct NvtxImporterError(String);

impl NvtxImporterError {
    pub fn new(error: impl fmt::Display) -> Self {
        Self(error.to_string())
    }
}

impl fmt::Display for NvtxImporterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for NvtxImporterError {}

pub type NvtxImporterResult<T> = Result<T, NvtxImporterError>;

/// Loads only one context's NVTX entity stream. `None` is a cacheable absence;
/// `Some(Vec::new())` is a present but empty stream.
pub type NvtxImporterFn =
    dyn Fn(Uuid) -> NvtxImporterResult<Option<Vec<Event<NvtxEventEntity>>>> + Send + Sync;

/// Import one context's NVTX stream from the standard filesystem layout.
pub fn import_context_events(
    root: &Path,
    context_id: Uuid,
) -> NvtxImporterResult<Option<Vec<Event<NvtxEventEntity>>>> {
    let context_dir = root.join(context_id.to_string());
    let stream_dir = context_dir.join(<NvtxEventEntity as EntityEvent>::NAME);
    if !stream_dir.is_dir() {
        return Ok(None);
    }
    let mut stream_entries = std::fs::read_dir(&stream_dir).map_err(NvtxImporterError::new)?;
    match stream_entries.next() {
        None => return Ok(Some(Vec::new())),
        Some(Err(error)) => return Err(NvtxImporterError::new(error)),
        Some(Ok(_)) => {}
    }
    let format = Format::detect(&context_dir)
        .ok_or_else(|| NvtxImporterError::new("unable to detect context stream format"))?;
    let importer = <ImporterOptions as ImporterProvider<NvtxEventEntity>>::create_importer(
        &ImporterOptions::FileSystem(filesystem::importer::Options {
            format,
            path: stream_dir,
        }),
    )
    .map_err(NvtxImporterError::new)?;
    importer
        .collect::<quent_io::ImporterResult<Vec<_>>>()
        .map(Some)
        .map_err(NvtxImporterError::new)
}

enum CachedModel {
    Absent,
    Present(Box<CachedNvtx>),
}

struct CachedNvtx {
    model: NvtxModel,
    catalogs: SyncCache<u64, Arc<NvtxCatalog>>,
}

impl CachedNvtx {
    fn new(model: NvtxModel) -> Self {
        Self {
            model,
            catalogs: SyncCache::builder()
                .max_capacity(MAX_CATALOG_ORIGINS)
                .build(),
        }
    }

    fn catalog(&self, query_start: u64) -> Arc<NvtxCatalog> {
        self.catalogs.get_with(query_start, || {
            Arc::new(NvtxCatalog::from_model(&self.model, query_start))
        })
    }
}

#[derive(Clone)]
struct NvtxModelCache {
    models: AsyncCache<Uuid, Arc<CachedModel>>,
    importer: Arc<NvtxImporterFn>,
}

impl NvtxModelCache {
    fn new(importer: Box<NvtxImporterFn>) -> Self {
        Self {
            models: AsyncCache::builder()
                .max_capacity(128)
                .time_to_idle(Duration::from_hours(24))
                .build(),
            importer: Arc::from(importer),
        }
    }

    async fn get(&self, context_id: Uuid) -> Result<Arc<CachedModel>, NvtxServerError> {
        let importer = Arc::clone(&self.importer);
        self.models
            .entry(context_id)
            .or_try_insert_with(async move {
                tokio::task::spawn_blocking(move || match importer(context_id) {
                    Ok(Some(events)) => {
                        let model = NvtxModelBuilder::build(events);
                        Ok(Arc::new(CachedModel::Present(Box::new(CachedNvtx::new(
                            model,
                        )))))
                    }
                    Ok(None) => Ok(Arc::new(CachedModel::Absent)),
                    Err(error) => Err(NvtxServerError::Internal(error.to_string())),
                })
                .await
                .map_err(|error| NvtxServerError::Internal(error.to_string()))?
            })
            .await
            .map(|entry| entry.into_value())
            .map_err(|error: Arc<NvtxServerError>| (*error).clone())
    }
}

#[derive(Clone)]
struct NvtxState {
    cache: NvtxModelCache,
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

async fn run_model_task<T>(task: impl FnOnce() -> T + Send + 'static) -> Result<T, NvtxServerError>
where
    T: Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|error| NvtxServerError::Internal(error.to_string()))
}

async fn catalog(
    State(state): State<NvtxState>,
    AxumPath(context_id): AxumPath<Uuid>,
    Query(origin): Query<NvtxTimeOrigin>,
) -> Result<Json<NvtxCatalog>, NvtxServerError> {
    let cached_model = state.cache.get(context_id).await?;
    run_model_task(move || match cached_model.as_ref() {
        CachedModel::Absent => Err(NvtxServerError::NotFound),
        CachedModel::Present(cached) => Ok(Json((*cached.catalog(origin.query_start)).clone())),
    })
    .await?
}

async fn viewport(
    State(state): State<NvtxState>,
    AxumPath(context_id): AxumPath<Uuid>,
    Query(origin): Query<NvtxTimeOrigin>,
    Json(request): Json<NvtxViewportRequest>,
) -> Result<Json<NvtxViewportResponse>, NvtxServerError> {
    validate_filter_count(&request)?;
    let cached_model = state.cache.get(context_id).await?;
    run_model_task(move || match cached_model.as_ref() {
        CachedModel::Absent => Err(NvtxServerError::NotFound),
        CachedModel::Present(cached) => {
            let catalog = cached.catalog(origin.query_start);
            NvtxViewportResponse::from_model_with_catalog(&cached.model, &catalog, request)
                .map(Json)
                .map_err(|error| NvtxServerError::BadRequest(error.to_string()))
        }
    })
    .await?
}

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

/// Context-keyed NVTX API routes, ready to merge into the host service before
/// its common CORS and embedded-UI fallback layers.
///
/// Catalog and viewport requests require a `query_start` query parameter in
/// Unix nanoseconds. Public viewport times are seconds relative to that origin.
pub fn routes(importer: Box<NvtxImporterFn>) -> Router {
    let state = NvtxState {
        cache: NvtxModelCache::new(importer),
    };
    Router::new()
        .route("/api/nvtx/contexts/{context_id}/catalog", get(catalog))
        .route("/api/nvtx/contexts/{context_id}/viewport", post(viewport))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};
    use tempfile::tempdir;
    use tower::ServiceExt;

    use super::*;

    const QUERY_START: u64 = 2_000_000_000;
    const RANGE_END: u64 = 3_000_000_000;

    fn range_events(context_id: Uuid) -> Vec<Event<NvtxEventEntity>> {
        let attributes = NvtxEventAttributes {
            message: Some(NvtxMessage::String("work".to_owned())),
            ..Default::default()
        };
        vec![
            Event::new(
                context_id,
                QUERY_START,
                NvtxEventEntity(NvtxEvent::RangeStart {
                    domain: 4,
                    range_id: 9,
                    attributes,
                }),
            ),
            Event::new(
                context_id,
                RANGE_END,
                NvtxEventEntity(NvtxEvent::RangeEnd {
                    domain: 4,
                    range_id: 9,
                }),
            ),
        ]
    }

    #[tokio::test(flavor = "current_thread")]
    async fn model_work_runs_off_the_async_executor() {
        let executor_thread = std::thread::current().id();
        let model_thread = run_model_task(|| std::thread::current().id())
            .await
            .unwrap();

        assert_ne!(model_thread, executor_thread);
    }

    #[tokio::test]
    async fn model_and_query_relative_catalogs_are_cached_end_to_end() {
        let present = Uuid::from_u128(1);
        let loads = Arc::new(AtomicUsize::new(0));
        let importer_loads = Arc::clone(&loads);
        let app = routes(Box::new(move |context_id| {
            importer_loads.fetch_add(1, Ordering::SeqCst);
            Ok((context_id == present).then(|| range_events(context_id)))
        }));

        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/{present}/catalog?query_start={QUERY_START}"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let catalog: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(catalog["trace_start"], 0.0);
        assert_eq!(catalog["trace_end"], 1.0);
        assert!(catalog.get("query_start").is_none());

        let alternate_origin = QUERY_START + 500_000_000;
        let alternate = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/{present}/catalog?query_start={alternate_origin}"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        let body = to_bytes(alternate.into_body(), usize::MAX).await.unwrap();
        let alternate: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(alternate["trace_start"], -0.5);
        assert_eq!(alternate["trace_end"], 0.5);

        let request = NvtxViewportRequest {
            viewport: nvtx_ui::NvtxViewportWindow {
                start: 0.0,
                end: 1.0,
            },
            selections: vec![nvtx_ui::NvtxDomainSelection {
                domain_id: 4,
                category_ids: vec![],
                include_uncategorized: true,
            }],
        };
        let response = app
            .oneshot(
                Request::post(format!(
                    "/api/nvtx/contexts/{present}/viewport?query_start={QUERY_START}"
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let viewport: NvtxViewportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(viewport.statistics[0].total_duration, 1.0);
        assert_eq!(
            loads.load(Ordering::SeqCst),
            1,
            "one reconstruction per context"
        );
    }

    #[tokio::test]
    async fn absent_and_empty_streams_are_distinct() {
        let empty = Uuid::from_u128(2);
        let absent = Uuid::from_u128(3);
        let app = routes(Box::new(move |context_id| {
            Ok((context_id == empty).then(Vec::new))
        }));

        let empty_response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/{empty}/catalog?query_start={QUERY_START}"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(empty_response.status(), StatusCode::OK);

        let absent_response = app
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/{absent}/catalog?query_start={QUERY_START}"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(absent_response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn filesystem_importer_recognizes_an_empty_present_stream() {
        let root = tempdir().unwrap();
        let context_id = Uuid::from_u128(8);
        std::fs::create_dir_all(
            root.path()
                .join(context_id.to_string())
                .join(<NvtxEventEntity as EntityEvent>::NAME),
        )
        .unwrap();

        let imported = import_context_events(root.path(), context_id).unwrap();
        assert!(imported.is_some_and(|events| events.is_empty()));
    }

    #[tokio::test]
    async fn invalid_selector_is_a_bounded_bad_request() {
        let present = Uuid::from_u128(4);
        let app = routes(Box::new(move |context_id| {
            Ok((context_id == present).then(|| range_events(context_id)))
        }));
        let request = NvtxViewportRequest {
            viewport: nvtx_ui::NvtxViewportWindow {
                start: 0.0,
                end: 1.0,
            },
            selections: vec![nvtx_ui::NvtxDomainSelection {
                domain_id: 4,
                category_ids: vec![],
                include_uncategorized: false,
            }],
        };
        let response = app
            .oneshot(
                Request::post(format!(
                    "/api/nvtx/contexts/{present}/viewport?query_start={QUERY_START}"
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_paths_and_oversized_requests_are_rejected() {
        let present = Uuid::from_u128(5);
        let app = routes(Box::new(move |context_id| {
            Ok((context_id == present).then(|| range_events(context_id)))
        }));

        let invalid_uuid = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/not-a-uuid/catalog?query_start={QUERY_START}"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid_uuid.status(), StatusCode::BAD_REQUEST);

        let missing_origin = app
            .clone()
            .oneshot(
                Request::get(format!("/api/nvtx/contexts/{present}/catalog"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_origin.status(), StatusCode::BAD_REQUEST);

        let oversized = app
            .oneshot(
                Request::post(format!(
                    "/api/nvtx/contexts/{present}/viewport?query_start={QUERY_START}"
                ))
                .header("content-type", "application/json")
                .body(Body::from(vec![b' '; MAX_BODY_BYTES + 1]))
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn importer_failures_are_redacted_and_isolated_by_context() {
        let failing = Uuid::from_u128(6);
        let present = Uuid::from_u128(7);
        let app = routes(Box::new(move |context_id| {
            if context_id == failing {
                Err(NvtxImporterError::new(
                    "/secret/capture/path could not be read",
                ))
            } else {
                Ok((context_id == present).then(|| range_events(context_id)))
            }
        }));

        let failed = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/{failing}/catalog?query_start={QUERY_START}"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let failed_body = to_bytes(failed.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&failed_body[..], b"NVTX data could not be loaded");

        let healthy = app
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/{present}/catalog?query_start={QUERY_START}"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(healthy.status(), StatusCode::OK);
    }
}
