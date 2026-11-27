// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_store_build::{EntityStoreImplementations, EventOptions, Options, generate};

mod schema;

fn main() {
    println!("cargo:rerun-if-changed=schema.rs");
    let schema = schema::schema();
    generate(
        &schema,
        &Options {
            events: EventOptions {
                serde: true,
                ..Options::default().events
            },
            filesystem: true,
            file_name: Some("model.rs".to_owned()),
            ..Options::default()
        },
    )
    .unwrap();

    let demo_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../instrumentation-build/example/model.yaml");
    println!("cargo:rerun-if-changed={}", demo_path.display());
    let demo = quent_yaml::parse_from_file(&demo_path).unwrap();
    generate(&demo.schema, &Options::default()).unwrap();

    generate(
        &schema::tree_schema(),
        &Options {
            events: EventOptions {
                serde: false,
                combined_event: false,
                event_derives: &["Clone"],
                ..Options::default().events
            },
            filesystem: false,
            file_name: Some("tree_model.rs".to_owned()),
            ..Options::default()
        },
    )
    .unwrap();

    generate(
        &quent_schema::test_utils::schema(
            "Plain",
            [quent_schema::test_utils::entity(
                "Task",
                [quent_schema::test_utils::event("parent_id", [])],
            )],
            [],
        ),
        &Options {
            events: EventOptions {
                serde: false,
                combined_event: false,
                ..Options::default().events
            },
            filesystem: false,
            file_name: Some("plain_model.rs".to_owned()),
            ..Options::default()
        },
    )
    .unwrap();

    // Compile each event/store combination; filesystem loading also needs serde and combined events.
    let mut modules = String::new();
    for serde in [false, true] {
        for native in [false, true] {
            for combined_event in [false, true] {
                let filesystems: &[bool] = if serde && combined_event && !native {
                    &[false, true]
                } else {
                    &[false]
                };
                for filesystem in filesystems {
                    let name = format!(
                        "serde_{serde}_native_{native}_combined_{combined_event}_filesystem_{filesystem}"
                    );
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
                            filesystem: *filesystem,
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
        }
    }
    std::fs::write(
        Options::default().out_dir.join("compile_options.rs"),
        modules,
    )
    .unwrap();
}
