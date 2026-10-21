// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use quent_store_build::{EventOptions, Options, generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../instrumentation-build/tests/external/application.yaml");
    println!("cargo:rerun-if-changed={}", path.display());
    let application = quent_yaml::parse_from_file(path)?;
    generate(
        &application.schema,
        &Options {
            events: EventOptions::default(),
            filesystem: false,
            ..Options::default()
        },
    )?;
    Ok(())
}
