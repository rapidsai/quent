// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use quent_bench_python_common::{BenchResult, measure};
use quent_bench_types::{CaseResult, Framework, Language, MeasurementArgs};

#[derive(Parser)]
struct Args {
    #[command(flatten)]
    workload: MeasurementArgs,
}

fn main() -> BenchResult<()> {
    let workload = Args::parse().workload;
    let measurement = measure(
        workload,
        include_str!("../measure.py"),
        serde_json::Value::Null,
    )?;
    let mut result: CaseResult<Framework, &str, &str> = CaseResult::try_new(
        Framework::EmptyLoopPython,
        Language::Python,
        None,
        None,
        workload,
        measurement.thread_batch_elapsed_ns,
        None,
    )?;
    result.child_pid = measurement.child_pid;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
