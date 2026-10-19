// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_store_build::{EntityStoreImplementations, EventOptions, Options, generate};

mod schema;

fn main() {
    println!("cargo:rerun-if-changed=schema.rs");
    let schema = schema::schema();
    let serde = cfg!(feature = "serde");
    generate(
        &schema,
        &Options {
            events: EventOptions {
                serde,
                ..Options::default().events
            },
            filesystem: serde,
            file_name: Some("model.rs".to_owned()),
            ..Options::default()
        },
    )
    .unwrap();

    // The demo schema is used by filesystem-based model-store tests, which
    // require Serde. Without Serde, the fixture schema still covers model-store
    // compilation and all entity-store runtime tests.
    if serde {
        let demo_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../instrumentation-build/example/model.yaml");
        println!("cargo:rerun-if-changed={}", demo_path.display());
        let demo = quent_yaml::parse_from_file(&demo_path).unwrap();
        generate(&demo.schema, &Options::default()).unwrap();
    }

    let mut modules = String::new();
    // Compile every other combination of native stores and loading options.
    // Filesystem loading requires Serde and combined events; the full build is
    // above.
    for native in [false, true] {
        for (combined_event, filesystem) in [(false, false), (true, false), (true, true)] {
            if filesystem && !serde {
                continue;
            }
            if native && combined_event && filesystem == serde {
                continue;
            }
            let name = format!("native_{native}_combined_{combined_event}_filesystem_{filesystem}");
            generate(
                &schema,
                &Options {
                    events: EventOptions {
                        debug: true,
                        serde,
                        combined_event,
                        ..Options::default().events
                    },
                    entity_store: EntityStoreImplementations { native },
                    filesystem,
                    file_name: Some(format!("{name}.rs")),
                    ..Options::default()
                },
            )
            .unwrap();
            modules.push_str(&format!(
                "mod {name} {{ include!(concat!(env!(\"OUT_DIR\"), \"/{name}.rs\")); }}\n"
            ));
        }
    }
    std::fs::write(
        Options::default().out_dir.join("compile_options.rs"),
        modules,
    )
    .unwrap();
    println!("cargo:rerun-if-changed=log.yaml");
    let log_schema = quent_yaml::parse_from_file(std::path::Path::new("log.yaml"))
        .unwrap()
        .schema;
    for (file_name, native) in [("log.rs", true), ("log_events_only.rs", false)] {
        generate(
            &log_schema,
            &Options {
                events: EventOptions {
                    serde,
                    combined_event: false,
                    ..Options::default().events
                },
                entity_store: EntityStoreImplementations { native },
                filesystem: false,
                file_name: Some(file_name.to_owned()),
                ..Options::default()
            },
        )
        .unwrap();
    }
    let namespaced_log =
        quent_log::LogEntityBuilder::new(quent_schema::test_utils::path("Nested::Log"))
            .with_levels(["fine", "critical"].map(|name| quent_log::LevelDecl {
                name: quent_schema::test_utils::ident(name),
                annotations: quent_schema::Annotations::default(),
                attributes: Vec::new(),
            }))
            .build()
            .unwrap();
    generate(
        &quent_schema::test_utils::schema("NamespacedLog", [namespaced_log], []),
        &Options {
            events: EventOptions {
                serde,
                ..Options::default().events
            },
            entity_store: EntityStoreImplementations { native: false },
            filesystem: false,
            file_name: Some("namespaced_log.rs".to_owned()),
            ..Options::default()
        },
    )
    .unwrap();
}
