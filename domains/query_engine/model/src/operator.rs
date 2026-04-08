// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Operator entity: sinks, sources, or transforms data within a plan.

use quent_attributes::Attribute;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, quent_model::Event, Deserialize, Serialize)]
pub struct Declaration {
    pub plan_id: quent_model::Ref<super::plan::Plan>,
    pub parent_operator_ids: Vec<Uuid>,
    pub instance_name: String,
    pub type_name: String,
    pub custom_attributes: Vec<Attribute>,
}

#[derive(Debug, quent_model::Event, Deserialize, Serialize)]
pub struct Statistics {
    pub custom_attributes: Vec<Attribute>,
}

#[derive(quent_model::Entity)]
#[resource_group]
pub struct Operator {
    pub declaration: quent_model::EmitOnce<Declaration>,
    pub statistics: quent_model::EmitOnce<Statistics>,
}
