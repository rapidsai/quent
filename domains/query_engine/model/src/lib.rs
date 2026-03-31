// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Query engine domain model.
//!
//! Defines entities, FSMs, and event types for the query engine domain using
//! quent-model proc macros. This is the single source of truth for all query
//! engine event types.

#[allow(unused_imports)]
use quent_model::prelude::*;

pub mod engine;
pub mod operator;
pub mod plan;
pub mod port;
pub mod query;
pub mod query_group;
pub mod worker;

use serde::{Deserialize, Serialize};

/// Top-level event enum for all query engine entities.
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

/// Model definition type for the query engine domain.
pub type QueryEngineModelDef = Model<(
    query::Query,
    engine::Engine,
    worker::Worker,
    query_group::QueryGroup,
    plan::Plan,
    operator::Operator,
    port::Port,
)>;
