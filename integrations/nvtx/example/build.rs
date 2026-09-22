// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=model.yaml");
    let parsed = quent_yaml::parse_from_file("model.yaml")?;
    quent_instrumentation_build::generate(
        &parsed.schema,
        &quent_instrumentation_build::Options {
            serde: true,
            event_derives: &["Clone"],
            record_derives: &["Clone"],
            umbrella_event: true,
            nvtx_capture: true,
            file_name: Some("instrumentation.rs".into()),
            ..Default::default()
        },
    )?;
    quent_store_build::generate(
        &parsed.schema,
        &quent_store_build::Options {
            file_name: Some("store.rs".into()),
            ..Default::default()
        },
    )?;
    Ok(())
}
