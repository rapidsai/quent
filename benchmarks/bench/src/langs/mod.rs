// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{num::NonZeroUsize, path::PathBuf, process::Command};

use quent_bench_types::Language;

use crate::{
    BenchResult, SharedArgs,
    case::{CaseRunner, run_child},
    progress::BuildProgress,
    report::CaseResult,
};

/// Builds an implementation in release mode and returns its executable path.
///
/// The Cargo subprocess inherits the caller's environment, including an active Pixi environment.
pub(crate) fn binary(package: &str, progress: &mut BuildProgress) -> BenchResult<PathBuf> {
    progress.started(package);
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    if package.starts_with("quent-bench-python-") {
        command.env(
            "PYO3_PYTHON",
            std::env::var_os("PYO3_PYTHON").unwrap_or_else(|| "python3".into()),
        );
        command.env("PYO3_BUILD_EXTENSION_MODULE", "1");
    }
    let output = command
        .args([
            "build",
            "--release",
            "-p",
            package,
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "failed to build {package}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let mut executable = None;
    for line in output.stdout.split(|byte| *byte == b'\n') {
        if let Ok(message) = serde_json::from_slice::<serde_json::Value>(line)
            && message["reason"] == "compiler-artifact"
            && message["target"]["name"] == package
            && let Some(path) = message["executable"].as_str()
        {
            executable = Some(PathBuf::from(path));
        }
    }
    let executable = executable.ok_or("Cargo did not report the implementation binary")?;
    progress.completed(package);
    Ok(executable)
}

/// Defines one empty-loop case for each requested thread count.
pub(crate) fn empty_loop_cases(
    language: Language,
    shared: &SharedArgs,
    progress: &mut BuildProgress,
) -> BenchResult<Vec<Box<dyn CaseRunner>>> {
    let executable = binary(
        &format!("quent-bench-{}-empty-loop", language.as_ref()),
        progress,
    )?;
    let mut cases = Vec::with_capacity(shared.threads.len());
    for threads in &shared.threads {
        cases.push(Box::new(EmptyLoopCase {
            language,
            executable: executable.clone(),
            threads: *threads,
        }) as Box<dyn CaseRunner>);
    }
    Ok(cases)
}

struct EmptyLoopCase {
    language: Language,
    executable: PathBuf,
    threads: NonZeroUsize,
}

impl CaseRunner for EmptyLoopCase {
    fn label(&self) -> String {
        format!(
            "empty-loop, {}, threads={}",
            self.language.as_ref(),
            self.threads
        )
    }

    fn run(&self, shared: &SharedArgs) -> BenchResult<CaseResult> {
        let mut command = Command::new(&self.executable);
        shared.apply_workload(&mut command, self.threads);
        run_child(command)
    }
}
