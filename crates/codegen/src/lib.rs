// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Code generation from Quent model definitions.
//!
//! Takes a `ModelBuilder` (populated by derive macros or deserialized from
//! YAML/JSON) and emits target-language code.

pub mod cxx_bridge;

use quent_model::ModelBuilder;

/// Configuration for the CXX bridge backend.
pub struct CxxOptions {
    /// C++ namespace for generated types (e.g., "myapp::quent").
    pub namespace: String,
    /// The Rust crate name (used for CXX include paths).
    pub crate_name: String,
    /// Path prefix for bridge modules within the crate (e.g., "src/bridge").
    pub bridge_path: String,
    /// Rust path to the model crate (e.g., "quent_cpp_example_model").
    pub model_crate: String,
    /// The top-level event enum type path (e.g., "quent_cpp_example_model::ExampleEvent").
    pub event_type: String,
}

impl Default for CxxOptions {
    fn default() -> Self {
        Self {
            namespace: "quent".to_string(),
            crate_name: "instrumentation".to_string(),
            bridge_path: ".".to_string(),
            model_crate: "model".to_string(),
            event_type: "model::Event".to_string(),
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
