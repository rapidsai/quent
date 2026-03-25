// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_attributes::Attribute;
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Init {
    pub implementation: Option<EngineImplementationAttributes>,
    pub instance_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum EngineEvent {
    Init(Init),
    Exit,
}
