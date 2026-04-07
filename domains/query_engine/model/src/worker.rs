// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Worker entity: responsible for executing plans.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, quent_model::Event, Deserialize, Serialize)]
pub struct Init {
    pub parent_engine_id: Uuid,
    pub instance_name: String,
}

#[derive(Debug, quent_model::Event, Deserialize, Serialize)]
pub struct Exit;

#[derive(quent_model::Entity)]
#[resource_group]
pub struct Worker {
    pub init: quent_model::EmitOnce<Init>,
    pub exit: quent_model::EmitOnce<Exit>,
}
