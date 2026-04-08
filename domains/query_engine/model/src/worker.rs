// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Worker entity: responsible for executing plans.

use quent_model::{EmitOnce, Entity, Event};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, Event, Deserialize, Serialize)]
pub struct Init {
    pub parent_engine_id: Uuid,
    pub instance_name: String,
}

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Exit;

#[derive(Entity)]
#[resource_group]
pub struct Worker {
    pub init: EmitOnce<Init>,
    pub exit: EmitOnce<Exit>,
}
