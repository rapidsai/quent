// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use ts_rs::TS;

use crate::FiniteStateMachine;

/// A ranked, paged list of entities.
#[derive(TS, Debug, Clone, Serialize)]
pub struct EntityListResponse {
    // TODO(johanpel): generalize to other entity types, but only FSMs are
    // represented today.
    pub items: Vec<FiniteStateMachine>,
    /// The count of entities matching the filter before paging.
    pub total: u32,
}
