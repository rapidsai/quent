// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

mod measure;
mod models;

use clap::Parser;
use quent_bench_quent_common::Exporter;
use quent_bench_rust_common::BenchResult;
use quent_bench_types::{CaseResult as SharedCaseResult, EventShape, Framework, MeasurementArgs};

type CaseResult = SharedCaseResult<Framework, Exporter, EventShape>;

#[derive(Parser)]
struct Args {
    #[arg(long, value_enum)]
    event_shape: EventShape,
    #[command(flatten)]
    workload: MeasurementArgs,
    #[command(flatten)]
    quent: QuentArgs,
}

#[derive(clap::Args)]
struct QuentArgs {
    #[arg(long, value_enum)]
    exporter: Exporter,
}

fn main() -> BenchResult<()> {
    let args = Args::parse();
    let result = measure::run_case(args.quent.exporter, args.event_shape, args.workload)?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
