// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use quent_bench_cpp_common::{BenchResult, ffi};
use quent_bench_types::{CaseResult, Framework, Language, MeasurementArgs};

#[derive(Parser)]
struct Args {
    #[command(flatten)]
    workload: MeasurementArgs,
}

fn main() -> BenchResult<()> {
    let workload = Args::parse().workload;
    let measurement = ffi::measure_empty(&workload.try_into()?)?;
    let result: CaseResult<Framework, &str, &str> = CaseResult::try_new(
        Framework::EmptyLoopCpp,
        Language::Cpp,
        None,
        None,
        workload,
        measurement.thread_durations(workload)?,
        None,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
