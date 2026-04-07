// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! QueryGroup entity: encapsulates a set of queries.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, quent_model::Event, Serialize, Deserialize)]
pub struct Declaration {
    pub instance_name: String,
    pub engine_id: Uuid,
}

#[derive(quent_model::Entity)]
#[resource_group]
pub struct QueryGroup {
    #[event]
    pub declaration: Declaration,
}
