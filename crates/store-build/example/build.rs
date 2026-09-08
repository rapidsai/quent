// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates store types from the schema shared with the instrumentation example.

use std::path::Path;

use quent_store_build::{Options, generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../instrumentation-build/example/model.yaml");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", model.display());

    let parsed = quent_yaml::parse_from_file(&model)?;
    for warning in &parsed.warnings {
        println!("cargo:warning={warning}");
    }

    let options = Options {
        // Generate `DemoEvent` so the example can load all model events through
        // one iterator. Entity-specific loading does not require this option.
        umbrella_event: true,
        ..Options::default()
    };
    let generated = generate(&parsed.schema, &options)?;
    if !generated.warnings.is_empty() {
        println!("cargo:warning= {}", generated.warnings.join("\n"));
    }
    println!(
        "cargo:warning=stored-event retrieval API written to {}",
        generated.path.display()
    );
    Ok(())
}
