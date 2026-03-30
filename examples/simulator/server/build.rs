// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_query_engine_ui::QueryBundle;
use quent_simulator_ui::{EntityRef, QueryFilter, TaskFilter};
use quent_ui::timeline::{
    request::{BulkTimelineRequest, SingleTimelineRequest},
    response::{BulkTimelinesResponse, SingleTimelineResponse},
};
use ts_rs::TS;

const TS_OUT_DIR: &str = "./ts-bindings/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Export TypeScript bindings to ts-bindings directory
    <QueryBundle<EntityRef> as TS>::export_all_to(TS_OUT_DIR)?;

    <SingleTimelineRequest<QueryFilter, TaskFilter> as TS>::export_all_to(TS_OUT_DIR)?;
    <SingleTimelineResponse as TS>::export_all_to(TS_OUT_DIR)?;
    <BulkTimelineRequest<QueryFilter, TaskFilter> as TS>::export_all_to(TS_OUT_DIR)?;
    <BulkTimelinesResponse as TS>::export_all_to(TS_OUT_DIR)?;

    Ok(())
}
