// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Engine entity: top-level entry point and root resource group.

use quent_model::{Attributes, entity};
use serde::{Deserialize, Serialize};

/// Attributes describing details about the implementation of this Engine
#[derive(Debug, Default, Attributes, Deserialize, Serialize)]
pub struct EngineImplementationAttributes {
    /// The name of this Engine implementation, e.g. "SiriusDB", "Velox", "DataFusion", etc.
    pub name: Option<String>,
    /// The version of this Engine implementation, e.g. "13.3.7"
    pub version: Option<String>,
    /// Arbitrary attributes defined at run time.
    pub custom_attributes: quent_attributes::CustomAttributes,
}

#[derive(Debug, Default, Attributes, Deserialize, Serialize)]
pub struct Init {
    pub implementation: EngineImplementationAttributes,
    pub instance_name: Option<String>,
}

#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Exit;

// Engine is the root resource group.
entity! {
    Engine: ResourceGroup<Root = true> {
        declaration: init,
        events: {
            init: Init,
            exit: Exit,
        },
    }
}
