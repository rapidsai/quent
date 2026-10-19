// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

#[test]
fn builds_without_ui_dependencies() {
    let mut cargo = Command::new(env!("CARGO"));
    cargo.current_dir(env!("CARGO_MANIFEST_DIR")).args([
        "tree",
        "--no-default-features",
        "--locked",
        "--offline",
        "--edges",
        "normal,build",
        "--prefix",
        "none",
        "--format",
        "{p}",
    ]);
    if cfg!(feature = "analysis") {
        cargo.args(["--features", "analysis"]);
    }
    let output = cargo.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dependencies = String::from_utf8(output.stdout).unwrap();
    for package in [
        "ts-rs",
        "ts-rs-macros",
        "quent-ui",
        "quent-analyzer",
        "regex",
    ] {
        assert!(
            !dependencies
                .lines()
                .any(|line| line.split_ascii_whitespace().next() == Some(package)),
            "UI dependency {package} is enabled:\n{dependencies}"
        );
    }
}
