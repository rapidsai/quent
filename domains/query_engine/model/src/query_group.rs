// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! QueryGroup entity: encapsulates a set of queries.

use quent_model::quent_model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Declaration {
    pub instance_name: String,
    pub engine_id: Uuid,
}

#[quent_model(entity(events(Declaration)), resource_group)]
pub struct QueryGroup;
