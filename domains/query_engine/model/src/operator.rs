// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Operator entity: sinks, sources, or transforms data within a plan.

use quent_attributes::Attribute;
use quent_model::quent_model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct Declaration {
    pub plan_id: Uuid,
    pub parent_operator_ids: Vec<Uuid>,
    pub instance_name: String,
    pub type_name: String,
    pub custom_attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Statistics {
    pub custom_attributes: Vec<Attribute>,
}

#[quent_model(entity(events(Declaration, Statistics)), resource_group)]
pub struct Operator;
