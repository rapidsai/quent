// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{env, path::Path};

use quent_instrumentation_build::{Options, generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Path::new(env!("CARGO_MANIFEST_DIR")).join("../model.yaml");
    println!("cargo:rerun-if-changed={}", model.display());

    let parsed = quent_yaml::parse_from_file(model)?;
    for warning in &parsed.warnings {
        println!("cargo:warning={warning}");
    }

    generate(
        &parsed.schema,
        &Options {
            serde: true,
            file_name: Some("instrumentation.rs".to_owned()),
            nvtx_capture: env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
                && env::var("CARGO_CFG_TARGET_POINTER_WIDTH").as_deref() == Ok("64"),
            ..Options::default()
        },
    )?;
    Ok(())
}
