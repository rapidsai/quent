// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Engine entity: top-level entry point and root resource group.

use quent_attributes::Attribute;
use quent_model::quent_model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EngineImplementationAttributes {
    pub name: Option<String>,
    pub version: Option<String>,
    pub custom_attributes: Vec<Attribute>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Init {
    pub implementation: Option<EngineImplementationAttributes>,
    pub instance_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Exit;

/// Engine is the root resource group.
#[quent_model(entity(events(Init, Exit)), resource_group(root))]
pub struct Engine;
