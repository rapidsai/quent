// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use quent_instrumentation_build::{EventOptions, Options, generate};

fn main() {
    let model = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../example/model.yaml");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", model.display());

    let parsed = quent_yaml::parse_from_file(&model).unwrap();
    generate(
        &parsed.schema,
        &Options {
            events: EventOptions {
                serde: true,
                combined_event: true,
                ..Default::default()
            },
            generate_timestamped_methods: true,
            ..Options::default()
        },
    )
    .unwrap();

    let mut modules = String::new();
    // Compile the other supported combinations; the full configuration is generated above.
    for (serde, combined_event) in [(false, false), (false, true), (true, false)] {
        let name = format!("serde_{serde}_combined_{combined_event}");
        generate(
            &parsed.schema,
            &Options {
                events: EventOptions {
                    serde,
                    combined_event,
                    ..Default::default()
                },
                file_name: Some(format!("{name}.rs")),
                ..Options::default()
            },
        )
        .unwrap();
        modules.push_str(&format!(
            "mod {name} {{ include!(concat!(env!(\"OUT_DIR\"), \"/{name}.rs\")); }}\n"
        ));
    }
    std::fs::write(
        Options::default().out_dir.join("compile_options.rs"),
        modules,
    )
    .unwrap();
}
