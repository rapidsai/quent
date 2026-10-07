// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{convert::Infallible, hint::black_box};

use clap::Parser;
use quent_bench_rust_common::{BenchResult, measure_threads};
use quent_bench_types::{CaseResult, Framework, Language, MeasurementArgs};

#[derive(Parser)]
struct Args {
    #[command(flatten)]
    workload: MeasurementArgs,
}

fn main() -> BenchResult<()> {
    let args = Args::parse();
    let durations = measure_threads(
        args.workload,
        || (),
        |_| (),
        |_, ()| {
            black_box(());
            Ok::<(), Infallible>(())
        },
    )?;
    let result: CaseResult<Framework, &str, &str> = CaseResult::try_new(
        Framework::EmptyLoopRs,
        Language::Rust,
        None,
        None,
        args.workload,
        durations,
        None,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
