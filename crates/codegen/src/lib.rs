// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Code generation from Quent model definitions.
//!
//! Takes a `ModelBuilder` (populated by derive macros or deserialized from
//! YAML/JSON) and emits target-language code.

pub mod cxx_bridge;

use quent_model::{ModelBuilder, FsmDef, EntityDef, StateDef, AttributeDef, ValueType};

/// Configuration for the CXX bridge backend.
pub struct CxxOptions {
    /// C++ namespace for generated types (e.g., "myapp::telemetry").
    pub namespace: String,
}

impl Default for CxxOptions {
    fn default() -> Self {
        Self {
            namespace: "telemetry".to_string(),
        }
    }
}

/// Generate CXX bridge Rust source code from a model.
///
/// Returns a map of filename → source code content.
pub fn emit_cxx(model: &ModelBuilder, options: &CxxOptions) -> Vec<GeneratedFile> {
    cxx_bridge::emit(model, options)
}

/// A generated source file.
pub struct GeneratedFile {
    /// Filename (e.g., "engine.rs").
    pub name: String,
    /// Source code content.
    pub content: String,
}
