// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_attributes::Attribute;
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

#[derive(Debug, Deserialize, Serialize)]
pub enum OperatorEvent {
    Declaration(Declaration),
    Statistics(Statistics),
}
