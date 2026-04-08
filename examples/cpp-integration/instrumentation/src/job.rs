// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Job entity: groups tasks, root resource group.

use quent_model::{EmitOnce, Entity, Event};

#[derive(Debug, Event, serde::Serialize, serde::Deserialize)]
pub struct Submit {
    pub name: String,
    pub num_tasks: u32,
}

#[derive(Debug, Event, serde::Serialize, serde::Deserialize)]
pub struct Complete {
    pub metadata: quent_attributes::CustomAttributes,
}

#[derive(Entity)]
#[resource_group(root)]
pub struct Job {
    pub submit: EmitOnce<Submit>,
    pub complete: EmitOnce<Complete>,
}
