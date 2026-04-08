// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Simulator instrumentation: model definitions, event types, and context.

#[allow(unused_imports)]
use quent_model::prelude::*;

pub mod task;

#[derive(Entity)]
#[resource_group]
pub struct ThreadPool;

#[derive(Entity)]
#[resource_group]
pub struct Network;

pub use task::TaskEvent;

quent_model::define_model! {
    Simulator {
        root: quent_query_engine_model::engine::Engine,
        quent_query_engine_model::worker::Worker,
        quent_query_engine_model::query_group::QueryGroup,
        quent_query_engine_model::query::Query,
        quent_query_engine_model::plan::Plan,
        quent_query_engine_model::operator::Operator,
        quent_query_engine_model::port::Port,
        task::Task,
        ThreadPool,
        Network,
        quent_stdlib::Memory,
        quent_stdlib::Processor,
        quent_stdlib::Channel,
    }
}

quent_model::define_instrumentation!(Simulator);
