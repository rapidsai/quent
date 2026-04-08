// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Port entity: input or output of an operator.

use quent_model::{EmitOnce, Entity, Event, Ref};
use serde::{Deserialize, Serialize};

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Declaration {
    /// The ID of the operator this port belongs to.
    pub operator_id: Ref<super::operator::Operator>,
    /// The name of this port.
    pub instance_name: String,
}

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Statistics {
    pub custom_attributes: quent_attributes::CustomAttributes,
}

#[derive(Entity)]
#[resource_group]
pub struct Port {
    pub declaration: EmitOnce<Declaration>,
    pub statistics: EmitOnce<Statistics>,
}
