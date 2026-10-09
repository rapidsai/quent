// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::Path, process::Command};

use quent_store_build::{Options, StorageSet, generate};

#[path = "fixtures/schema.rs"]
mod model_schema;

#[test]
fn generated_entity_handles_compile_and_borrow_declared_field_types() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join("src");
    fs::create_dir(&source).unwrap();
    for (file_name, native) in [("model.rs", true), ("access_only.rs", false)] {
        generate(
            &model_schema::schema(),
            &Options {
                entity_storage: Some(StorageSet { native }),
                debug: native,
                combined_event: false,
                filesystem: false,
                out_dir: source.clone(),
                file_name: Some(file_name.to_owned()),
                ..Options::default()
            },
        )
        .unwrap();
    }
    fs::write(
        source.join("lib.rs"),
        include_str!("fixtures/entity_events.rs"),
    )
    .unwrap();

    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let events = crate_dir.join("../events");
    let store = crate_dir.join("../store");
    let manifest = fixture.path().join("Cargo.toml");
    fs::copy(
        crate_dir.join("../../Cargo.lock"),
        fixture.path().join("Cargo.lock"),
    )
    .unwrap();
    fs::write(
        &manifest,
        format!(
            r#"[package]
name = "quent-store-build-entity-events-test"
version = "0.0.0"
edition = "2024"

[workspace]

[dependencies]
quent-events = {{ path = {events:?}, features = ["serde"] }}
quent-store = {{ path = {store:?}, default-features = false }}
serde = {{ version = "1", features = ["derive"] }}
"#,
        ),
    )
    .unwrap();

    // A separate target directory avoids contending with the parent Cargo test process.
    let output = Command::new(env!("CARGO"))
        .args(["test", "--offline", "--manifest-path"])
        .arg(&manifest)
        .env(
            "CARGO_TARGET_DIR",
            crate_dir.join("../../target/tests/store-build"),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
