// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Operator entity: sinks, sources, or transforms data within a plan.

use quent_model::{Attributes, Ref, entity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Declaration {
    pub plan_id: Ref<super::plan::Plan>,
    pub parent_operator_ids: Vec<Ref<super::operator::Operator>>,
    pub instance_name: String,
    pub type_name: String,
    pub custom_attributes: quent_model::attributes::DynamicAttributes,
}

/// A producer-defined role for one of an operator's ports.
#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct PortRelation {
    pub port_id: Ref<super::port::Port>,
    pub role: String,
}

#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Statistics {
    pub information: Vec<super::information::InformationGroup>,
    pub port_relations: Vec<PortRelation>,
}

/// A timestamped, producer-defined observation about an operator.
#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Observation {
    pub kind: String,
    pub custom_attributes: quent_model::attributes::DynamicAttributes,
    pub port_relations: Vec<PortRelation>,
}

entity! {
    Operator: ResourceGroup {
        declaration: declaration,
        events: {
            declaration: Declaration,
            observation: Observation,
            statistics: Statistics,
        },
    }
}
