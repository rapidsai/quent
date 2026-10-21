// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX views of cached application analysis.

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use quent_nvtx_ui::{NvtxCatalog, NvtxViewportRequest, NvtxViewportResponse};
use quent_query_engine_server::{
    analyzer_cache::AnalyzerCache,
    error::{ServerError, ServerResult},
};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
struct TimeOrigin {
    query_start: u64,
}

async fn sinks(
    State(state): State<AnalyzerCache<SimulatorUiAnalyzer>>,
    Path(engine_id): Path<Uuid>,
) -> ServerResult<Json<Vec<Uuid>>> {
    let analyzer = state.get(engine_id).await?;
    let mut ids: Vec<_> = analyzer.nvtx_models().map(|(id, _)| id).collect();
    ids.sort_unstable();
    Ok(Json(ids))
}

async fn catalog(
    State(state): State<AnalyzerCache<SimulatorUiAnalyzer>>,
    Path((engine_id, sink_id)): Path<(Uuid, Uuid)>,
    Query(origin): Query<TimeOrigin>,
) -> Result<Json<NvtxCatalog>, (StatusCode, String)> {
    let analyzer = state.get(engine_id).await.map_err(server_error)?;
    tokio::task::spawn_blocking(move || {
        let (_, model) = analyzer
            .nvtx_models()
            .find(|(id, _)| *id == sink_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "NVTX sink not found".to_owned()))?;
        Ok(Json(NvtxCatalog::from_model(model, origin.query_start)))
    })
    .await
    .map_err(|error| server_error(error.into()))?
}

async fn viewport(
    State(state): State<AnalyzerCache<SimulatorUiAnalyzer>>,
    Path((engine_id, sink_id)): Path<(Uuid, Uuid)>,
    Query(origin): Query<TimeOrigin>,
    Json(request): Json<NvtxViewportRequest>,
) -> Result<Json<NvtxViewportResponse>, (StatusCode, String)> {
    if request.selections.len() > 256
        || request
            .selections
            .iter()
            .map(|selection| selection.category_ids.len())
            .sum::<usize>()
            > 4096
    {
        return Err((StatusCode::BAD_REQUEST, "too many NVTX filters".to_owned()));
    }
    let analyzer = state.get(engine_id).await.map_err(server_error)?;
    tokio::task::spawn_blocking(move || {
        let (_, model) = analyzer
            .nvtx_models()
            .find(|(id, _)| *id == sink_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "NVTX sink not found".to_owned()))?;
        let catalog = NvtxCatalog::from_model(model, origin.query_start);
        NvtxViewportResponse::from_model_with_catalog(model, &catalog, request)
            .map(Json)
            .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))
    })
    .await
    .map_err(|error| server_error(error.into()))?
}

fn server_error(error: ServerError) -> (StatusCode, String) {
    let message = error.to_string();
    (error.into(), message)
}

pub(crate) fn routes(state: AnalyzerCache<SimulatorUiAnalyzer>) -> Router {
    Router::new()
        .route("/{engine_id}/nvtx", get(sinks))
        .route("/{engine_id}/nvtx/{sink_id}/catalog", get(catalog))
        .route("/{engine_id}/nvtx/{sink_id}/viewport", post(viewport))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .with_state(state)
}

#[cfg(test)]
mod tests;
