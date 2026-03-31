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

// The top-level event enum and model type are auto-generated from the
// model component list.
quent_model::define_model! {
    pub QueryEngineModelDef(QueryEngineEvent) {
        Query: query::Query,
        Engine: engine::Engine,
        Worker: worker::Worker,
        QueryGroup: query_group::QueryGroup,
        Plan: plan::Plan,
        Operator: operator::Operator,
        Port: port::Port,
    }
}
