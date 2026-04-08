// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Operator entity: sinks, sources, or transforms data within a plan.

use quent_model::{EmitOnce, Entity, Event, Ref};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Declaration {
    pub plan_id: Ref<super::plan::Plan>,
    pub parent_operator_ids: Vec<Ref<super::operator::Operator>>,
    pub instance_name: String,
    pub type_name: String,
    pub custom_attributes: quent_attributes::CustomAttributes,
}

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Statistics {
    pub custom_attributes: quent_attributes::CustomAttributes,
}

#[derive(Entity)]
#[resource_group]
pub struct Operator {
    pub declaration: EmitOnce<Declaration>,
    pub statistics: EmitOnce<Statistics>,
}
