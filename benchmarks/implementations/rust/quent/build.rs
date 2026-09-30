// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use quent_instrumentation_build::{Options, generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    for name in ["empty", "u8", "u64", "short_string", "long_string", "all"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../models")
            .join(format!("{name}.yaml"));
        println!("cargo:rerun-if-changed={}", path.display());
        let parsed = quent_yaml::parse_from_file(path)?;
        for warning in parsed.warnings {
            println!("cargo:warning={warning}");
        }
        generate(
            &parsed.schema,
            &Options {
                file_name: Some(format!("{name}.rs")),
                serde: true,
                ..Options::default()
            },
        )?;
    }
    Ok(())
}
