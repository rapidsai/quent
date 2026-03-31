// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! QueryGroup entity: encapsulates a set of queries.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[quent_model::resource_group]
pub struct QueryGroup;

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryGroupEvent {
    pub instance_name: String,
    pub engine_id: Uuid,
}
