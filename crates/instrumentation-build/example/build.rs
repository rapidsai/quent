// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Parses the YAML model and generates the instrumentation library into
//! `OUT_DIR`.

use std::path::Path;

use quent_instrumentation_build::{EventOptions, GenerateInfo, Options, generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Path::new(env!("CARGO_MANIFEST_DIR")).join("model.yaml");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", model.display());

    // YAML source -> quent_schema::Schema. Errors carry file:line:column.
    let parsed = quent_yaml::parse_from_file(&model)?;
    // Constraints no validator handles (none in this model).
    for warning in &parsed.warnings {
        println!("cargo:warning={warning}");
    }

    // Schema -> generated Rust instrumentation source.
    let opts = Options {
        events: EventOptions {
            // Filesystem exporters serialize generated events.
            serde: true,
            // Generate `DemoEvent` so one typed callback can receive every
            // entity's events.
            combined_event: true,
            ..Default::default()
        },
        ..Options::default()
    };
    let GenerateInfo { path, warnings } = generate(&parsed.schema, &opts)?;

    if !warnings.is_empty() {
        println!("cargo:warning= {}", warnings.join("\n"));
    }
    println!(
        "cargo:warning=instrumentation library written to {}",
        path.display()
    );

    Ok(())
}
