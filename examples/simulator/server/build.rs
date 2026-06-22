// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_query_engine_ui::{OperatorFilter, QueryBundle, QueryFilter};
use quent_simulator_ui::EntityRef;
use quent_ui::timeline::{
    request::{BulkTimelineRequest, SingleTimelineRequest},
    response::{BulkTimelinesResponse, SingleTimelineResponse},
};
use ts_rs::{Config, TS};

const TS_OUT_DIR: &str = "./ts-bindings/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Export TypeScript bindings to ts-bindings directory
    let cfg = Config::new().with_out_dir(TS_OUT_DIR);

    <QueryBundle<EntityRef> as TS>::export_all(&cfg)?;

    <SingleTimelineRequest<QueryFilter, OperatorFilter> as TS>::export_all(&cfg)?;
    <SingleTimelineResponse as TS>::export_all(&cfg)?;
    <BulkTimelineRequest<QueryFilter, OperatorFilter> as TS>::export_all(&cfg)?;
    <BulkTimelinesResponse as TS>::export_all(&cfg)?;

    Ok(())
}
