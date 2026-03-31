// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Worker entity: responsible for executing plans.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[quent_model::resource_group]
pub struct Worker;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct WorkerInit {
    pub parent_engine_id: Uuid,
    pub instance_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum WorkerEvent {
    Init(WorkerInit),
    Exit,
}
