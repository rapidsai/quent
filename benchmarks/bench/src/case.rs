// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

use crate::{BenchResult, SharedArgs, report::CaseResult};

/// Runs one isolated benchmark case.
pub(crate) trait CaseRunner {
    /// Describes the case shown while its child process runs.
    fn label(&self) -> String;

    /// Runs the child process and returns its measurement.
    fn run(&self, shared: &SharedArgs) -> BenchResult<CaseResult>;
}

/// Runs a child process and reads its result.
pub(crate) fn run_child(mut command: Command) -> BenchResult<CaseResult> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "{command:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}
