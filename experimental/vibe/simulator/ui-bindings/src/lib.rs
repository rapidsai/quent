// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! TypeScript binding generation for the simulator UI.

use std::path::Path;

use nvtx_ui::{NvtxCatalog, NvtxViewportRequest, NvtxViewportResponse};
use quent_query_engine_ui::DataFlowTimelineBinned;
use quent_query_engine_ui::{EngineContexts, OperatorFilter, QueryBundle, QueryFilter};
use quent_simulator_ui::EntityRef;
use quent_ui::entities::{request::EntityListRequest, response::EntityListResponse};
use quent_ui::timeline::{
    categorical::CategoricalTimelineRequest,
    request::{BulkTimelineRequest, SingleTimelineRequest},
    response::{BulkTimelinesResponse, SingleTimelineResponse},
};
use ts_rs::{Config, TS};

/// Generates simulator UI TypeScript bindings in `output_dir`.
///
/// Existing contents of `output_dir` are removed before generation.
pub fn generate(output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir)?;
    }
    std::fs::create_dir_all(output_dir)?;
    let cfg = Config::new().with_out_dir(output_dir);

    <QueryBundle<EntityRef> as TS>::export_all(&cfg)?;

    <SingleTimelineRequest<QueryFilter, OperatorFilter> as TS>::export_all(&cfg)?;
    <SingleTimelineResponse as TS>::export_all(&cfg)?;
    <BulkTimelineRequest<QueryFilter, OperatorFilter> as TS>::export_all(&cfg)?;
    <BulkTimelinesResponse as TS>::export_all(&cfg)?;
    <CategoricalTimelineRequest<QueryFilter> as TS>::export_all(&cfg)?;
    <DataFlowTimelineBinned as TS>::export_all(&cfg)?;
    <EngineContexts as TS>::export_all(&cfg)?;
    <NvtxCatalog as TS>::export_all(&cfg)?;
    <NvtxViewportRequest as TS>::export_all(&cfg)?;
    <NvtxViewportResponse as TS>::export_all(&cfg)?;

    <EntityListRequest<QueryFilter, OperatorFilter> as TS>::export_all(&cfg)?;
    <EntityListResponse as TS>::export_all(&cfg)?;

    Ok(())
}
