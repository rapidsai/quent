// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Port entity: input or output of an operator.

use quent_attributes::Attribute;
use quent_model::quent_model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[quent_model(resource_group)]
pub struct Port;

#[derive(Debug, Deserialize, Serialize)]
pub struct Declaration {
    pub operator_id: Uuid,
    pub instance_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Statistics {
    pub custom_attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PortEvent {
    Declaration(Declaration),
    Statistics(Statistics),
}
