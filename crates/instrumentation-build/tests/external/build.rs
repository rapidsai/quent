// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

use quent_instrumentation_build::{EventOptions, Options, generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=predefined.yaml");
    let predefined = quent_yaml::parse_from_file(root.join("predefined.yaml"))?;
    generate(&predefined.schema, &Options::default())?;

    println!("cargo:rerun-if-changed=application.yaml");
    let application = quent_yaml::parse_from_file(root.join("application.yaml"))?;
    generate(
        &application.schema,
        &Options {
            events: EventOptions {
                // Tests collect payloads from all entities through ApplicationEvent.
                combined_event: true,
                ..EventOptions::default()
            },
            ..Options::default()
        },
    )?;

    let runtime = quent_events_build::Runtime {
        events: syn::parse_quote!(::quent_events),
        entity_marker: syn::parse_quote!(::quent_events::EntityMarker),
        serde: None,
    };
    let options = EventOptions {
        combined_event: true,
        ..EventOptions::default()
    };
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let source = std::fs::read_to_string(root.join("application.yaml"))?;
    for (file, source) in [
        ("application_event_only.rs", source.clone()),
        (
            "application_bad_name.rs",
            source.replace("  Annotations:", "  Annotationx:"),
        ),
    ] {
        let parsed = quent_yaml::parse_from_str(source, None)?;
        std::fs::write(
            out_dir.join(file),
            quent_events_build::generate_str(&parsed.schema, &options, &runtime)?,
        )?;
    }
    Ok(())
}
