// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const MANIFEST: &str = r#"
model = "model.yaml"
targets = ["rust"]

[package]
name = "my-instrumentation"
version = "0.1.0"
"#;

const MODEL: &str =
    "quent: alpha\nmodel: demo\nentities:\n  Task:\n    events:\n      started: {}\n";

fn check(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quent-codegen"))
        .current_dir(cwd)
        .arg("check")
        .args(args)
        .output()
        .unwrap()
}

#[track_caller]
fn assert_failure(output: &Output, expected: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains(expected), "expected {expected:?}: {stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn resolves_models_relative_to_manifest_and_writes_no_files() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project with spaces");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("quent.toml"), MANIFEST).unwrap();
    fs::write(project.join("model.yaml"), MODEL).unwrap();
    // A model in the working directory must not shadow the manifest's model.
    fs::write(dir.path().join("model.yaml"), "invalid").unwrap();

    for (cwd, args) in [
        (project.as_path(), vec![]),
        (
            dir.path(),
            vec!["--manifest-path", "project with spaces/quent.toml"],
        ),
    ] {
        let output = check(cwd, &args);
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("Valid:"));
    }
    assert_eq!(fs::read_dir(&project).unwrap().count(), 2);
    assert_eq!(
        fs::read_to_string(project.join("quent.toml")).unwrap(),
        MANIFEST
    );
    assert_eq!(
        fs::read_to_string(project.join("model.yaml")).unwrap(),
        MODEL
    );
}

#[test]
fn accepts_absolute_manifest_and_model_paths() {
    let dir = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let model = dir.path().join("model.yaml");
    fs::write(&model, MODEL).unwrap();
    let manifest = MANIFEST.replace(
        "\"model.yaml\"",
        &toml::Value::String(model.display().to_string()).to_string(),
    );
    let path = dir.path().join("quent.toml");
    fs::write(&path, manifest).unwrap();
    let output = check(
        elsewhere.path(),
        &["--manifest-path", path.to_str().unwrap()],
    );
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn rejects_invalid_manifests_before_loading_the_model() {
    let dir = tempfile::tempdir().unwrap();
    for (manifest, expected) in [
        ("model = [".into(), "TOML parse error"),
        (
            MANIFEST.replace("model = \"model.yaml\"", ""),
            "missing field `model`",
        ),
        (
            MANIFEST.replace("targets = [\"rust\"]", ""),
            "missing field `targets`",
        ),
        (
            "model = 'model.yaml'\ntargets = ['rust']".into(),
            "missing field `package`",
        ),
        (
            MANIFEST.replace("version = \"0.1.0\"", ""),
            "missing field `version`",
        ),
        (
            MANIFEST.replace("name = \"my-instrumentation\"", ""),
            "missing field `name`",
        ),
        (format!("typo = true\n{MANIFEST}"), "unknown field `typo`"),
        (format!("{MANIFEST}\ntypo = true"), "unknown field `typo`"),
        (
            MANIFEST.replace("[\"rust\"]", "[\"python\"]"),
            "unknown variant `python`",
        ),
        (
            MANIFEST.replace("[\"rust\"]", "[\"cpp\"]"),
            "unknown variant `cpp`",
        ),
        (
            MANIFEST.replace("[\"rust\"]", "[]"),
            "`targets` must not be empty",
        ),
        (
            MANIFEST.replace("[\"rust\"]", "[\"rust\", \"rust\"]"),
            "duplicates",
        ),
        (
            MANIFEST.replace("model.yaml", ""),
            "`model` must not be empty",
        ),
        (MANIFEST.replace("my-instrumentation", ""), "`package.name`"),
        (
            MANIFEST.replace("my-instrumentation", "bad name"),
            "`package.name`",
        ),
        (
            MANIFEST.replace("0.1.0", "0.1"),
            "invalid `package.version`",
        ),
    ] {
        fs::write(dir.path().join("quent.toml"), manifest).unwrap();
        let output = check(dir.path(), &[]);
        assert_failure(&output, expected);
        assert!(String::from_utf8_lossy(&output.stderr).contains("quent.toml"));
    }
}

#[test]
fn reports_missing_files_and_model_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    assert_failure(&check(dir.path(), &[]), "quent.toml");
    fs::write(dir.path().join("quent.toml"), MANIFEST).unwrap();
    assert_failure(&check(dir.path(), &[]), "model.yaml");

    for (model, expected) in [
        ("quent: alpha\nmodel: [\n", "model.yaml:2:"),
        (
            "quent: alpha\nmodel: demo\nentities:\n  Task: {}\n",
            "declares no events",
        ),
    ] {
        fs::write(dir.path().join("model.yaml"), model).unwrap();
        let output = check(dir.path(), &[]);
        assert_failure(&output, expected);
        assert!(String::from_utf8_lossy(&output.stderr).contains("model.yaml"));
    }
}

#[test]
fn warnings_do_not_fail_validation() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("quent.toml"), MANIFEST).unwrap();
    fs::write(
        dir.path().join("model.yaml"),
        format!("{MODEL}constraints:\n  example.unknown: null\n"),
    )
    .unwrap();
    let output = check(dir.path(), &[]);
    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning:") && stderr.contains("example.unknown"),
        "{stderr}"
    );
}

#[test]
fn readme_manifest_is_valid() {
    let output = check(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &["--manifest-path", "../../examples/readme/quent.toml"],
    );
    assert!(output.status.success(), "{output:?}");
}
