// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use ts_rs::TS;

use quent_time::TimeSec;

use crate::FiniteStateMachine;

/// An entity and its longest matching resource usage.
#[derive(TS, Debug, Clone, Serialize)]
pub struct EntityListItem {
    pub entity: FiniteStateMachine,
    pub usage_duration_s: TimeSec,
}

/// A ranked, paged list of entities.
#[derive(TS, Debug, Clone, Serialize)]
pub struct EntityListResponse {
    // TODO(johanpel): generalize to other entity types, but only FSMs are
    // represented today.
    pub items: Vec<EntityListItem>,
    /// The count of entities matching the filter before paging.
    pub total: u32,
}
