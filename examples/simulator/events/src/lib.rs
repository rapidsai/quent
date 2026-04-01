// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

pub use quent_simulator_model::task;
pub type TaskEvent = task::TaskEvent;

quent_model::define_model! {
    Simulator {
        quent_query_engine_model::engine::Engine,
        quent_query_engine_model::worker::Worker,
        quent_query_engine_model::query_group::QueryGroup,
        quent_query_engine_model::query::Query,
        quent_query_engine_model::plan::Plan,
        quent_query_engine_model::operator::Operator,
        quent_query_engine_model::port::Port,
        quent_simulator_model::task::Task,
        quent_stdlib::Memory,
        quent_stdlib::Processor,
        quent_stdlib::Channel,
    }
    extra {
        ResourceGroup: quent_events::resource::GroupEvent,
        Trace: quent_events::trace::TraceEvent,
    }
}
