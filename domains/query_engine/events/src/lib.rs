// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

pub mod engine;
pub mod operator;
pub mod plan;
pub mod port;
pub mod query;
pub mod query_group;
pub mod worker;

#[derive(Debug, Deserialize, Serialize)]
pub enum QueryEngineEvent {
    Engine(engine::EngineEvent),
    Worker(worker::WorkerEvent),
    QueryGroup(query_group::QueryGroupEvent),
    Query(query::QueryEvent),
    Plan(plan::PlanEvent),
    Operator(operator::OperatorEvent),
    Port(port::PortEvent),
}
