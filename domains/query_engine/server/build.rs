// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "ui")]
    build_ui()?;

    Ok(())
}

#[cfg(feature = "ui")]
fn build_ui() -> Result<(), Box<dyn std::error::Error>> {
    let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ui");
    let bindings_dir = ui_dir.join("generated/ts-bindings");
    let rerun_if_changed = |path: &std::path::Path| {
        println!("cargo:rerun-if-changed={}", path.display());
    };
    for path in [
        "src",
        "public",
        "index.html",
        "package.json",
        "pnpm-lock.yaml",
        "pnpm-workspace.yaml",
        "tsconfig.base.json",
        "tsconfig.json",
        "tsconfig.node.json",
        "tsr.config.json",
        "vite.config.ts",
    ] {
        rerun_if_changed(&ui_dir.join(path));
    }
    for package in ["client", "components", "hooks", "utils"] {
        let package_dir = ui_dir.join("packages/@quent").join(package);
        for path in ["src", "package.json", "tsconfig.json", "tsdown.config.ts"] {
            rerun_if_changed(&package_dir.join(path));
        }
    }

    let run_pnpm = |args: &[&str]| -> Result<(), Box<dyn std::error::Error>> {
        let output = std::process::Command::new("pnpm")
            .args(args)
            .current_dir(&ui_dir)
            .output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Vite prefixes warnings with "(!)".
        let has_warnings = stderr
            .lines()
            .any(|line| line.trim_start().starts_with("(!)"));
        if !output.status.success() || has_warnings {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                println!("cargo:warning=pnpm: {line}");
            }
            for line in stderr.lines() {
                println!("cargo:warning=pnpm: {line}");
            }
        }
        if !output.status.success() {
            return Err(format!("pnpm {} failed", args.join(" ")).into());
        }
        Ok(())
    };

    quent_simulator_ui_bindings::generate(&bindings_dir)?;
    run_pnpm(&["install", "--frozen-lockfile"])?;
    run_pnpm(&["ci:build"])?;

    Ok(())
}
