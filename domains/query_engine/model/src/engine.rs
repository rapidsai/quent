// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Engine entity: top-level entry point and root resource group.

use quent_attributes::Attribute;
use quent_model::{EmitOnce, Entity, Event};
use serde::{Deserialize, Serialize};

/// Attributes describing details about the implementation of this Engine
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EngineImplementationAttributes {
    /// The name of this Engine implementation, e.g. "SiriusDB", "Velox", "DataFusion", etc.
    pub name: Option<String>,
    /// The version of this Engine implementation, e.g. "13.3.7"
    pub version: Option<String>,
    /// Arbitrary attributes defined at run time.
    pub custom_attributes: Vec<Attribute>,
}

#[derive(Debug, Default, Event, Deserialize, Serialize)]
pub struct Init {
    pub implementation: Option<EngineImplementationAttributes>,
    pub instance_name: Option<String>,
}

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Exit;

/// Engine is the root resource group.
#[derive(Entity)]
#[resource_group(root)]
pub struct Engine {
    pub init: EmitOnce<Init>,
    pub exit: EmitOnce<Exit>,
}
