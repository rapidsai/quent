// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Port entity: input or output of an operator.

use quent_attributes::Attribute;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, quent_model::Event, Deserialize, Serialize)]
pub struct Declaration {
    /// The ID of the operator this port belongs to.
    pub operator_id: Uuid,
    /// The name of this port.
    pub instance_name: String,
}

#[derive(Debug, quent_model::Event, Deserialize, Serialize)]
pub struct Statistics {
    pub custom_attributes: Vec<Attribute>,
}

#[derive(quent_model::Entity)]
#[resource_group]
pub struct Port {
    pub declaration: quent_model::EmitOnce<Declaration>,
    pub statistics: quent_model::EmitOnce<Statistics>,
}
