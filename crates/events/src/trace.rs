// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Module containing events for run-time defined tracing

use quent_attributes::Attribute;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type SpanId = u64;

#[derive(Debug, Deserialize, Serialize)]
pub struct TraceInit {
    pub entity_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpanInit {
    pub id: SpanId,
    pub name: String,
    pub parent_id: Option<SpanId>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpanEnter {
    pub id: SpanId,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpanExit {
    pub id: SpanId,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpanClose {
    pub id: SpanId,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TraceEvent {
    /// Declare a trace entity.
    Init(TraceInit),
    /// Declare a span within the trace.
    Span(SpanInit),
    /// Enter a span within the trace.
    Enter(SpanEnter),
    /// Exit a span within the trace.
    Exit(SpanExit),
    /// Close a span within the trace.
    Close(SpanClose),
}
