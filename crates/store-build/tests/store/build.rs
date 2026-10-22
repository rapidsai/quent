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
}
