// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Query engine domain model.

use quent_model::model;

pub mod engine;
pub mod operator;
pub mod plan;
pub mod port;
pub mod query;
pub mod query_group;
pub mod worker;

model! {
    QueryEngine {
        root: engine::Engine,
        query::Query,
        worker::Worker,
        query_group::QueryGroup,
        plan::Plan,
        operator::Operator,
        port::Port,
    }
}
