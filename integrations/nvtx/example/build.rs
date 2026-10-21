// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use quent_instrumentation_build::{EventOptions, Options, generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=model.yaml");
    let model = Path::new(env!("CARGO_MANIFEST_DIR")).join("model.yaml");
    let parsed = quent_yaml::parse_from_file(model)?;
    generate(
        &parsed.schema,
        &Options {
            events: EventOptions {
                combined_event: true,
                ..EventOptions::default()
            },
            ..Options::default()
        },
    )?;
    Ok(())
}
