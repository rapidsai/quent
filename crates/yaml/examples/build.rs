// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use quent_instrumentation_build::{Options, generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for relative_path in [
        "minimal-model/model.yaml",
        "event-data/model.yaml",
        "dynamic-attributes/model.yaml",
        "repeated-events/model.yaml",
        "records/model.yaml",
        "untyped-entity-references/model.yaml",
        "entity-references/model.yaml",
        "scoped-references/model.yaml",
        "finite-state-machine/model.yaml",
        "fsm-self-loop/model.yaml",
        "unit-resource/model.yaml",
        "resource-capacity/model.yaml",
        "bounded-resource/model.yaml",
        "job-workload/model.yaml",
        "log-sink/model.yaml",
    ] {
        let model = root.join(relative_path);
        println!("cargo:rerun-if-changed={}", model.display());

        let parsed = quent_yaml::parse_from_file(&model)?;
        for warning in parsed.warnings {
            println!("cargo:warning={warning}");
        }

        generate(&parsed.schema, &Options::default())?;
    }

    Ok(())
}
