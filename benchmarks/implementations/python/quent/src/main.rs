// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use quent_bench_python_common::measure;
use quent_bench_quent_common::{BenchResult, Exporter, discarded_events};
use quent_bench_types::{CaseResult, EventShape, Framework, Language, MeasurementArgs};
use serde_json::json;

#[derive(Parser)]
struct Args {
    #[arg(long, value_enum)]
    event_shape: EventShape,
    #[arg(long, value_enum)]
    exporter: Exporter,
    #[command(flatten)]
    workload: MeasurementArgs,
}

fn main() -> BenchResult<()> {
    let args = Args::parse();
    let directory = args
        .exporter
        .file_format()
        .map(|_| tempfile::tempdir())
        .transpose()?;
    let root = directory.as_ref().map(|directory| directory.path());
    let library = std::env::current_exe()?.with_file_name(format!(
        "{}quent_bench_python{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX,
    ));
    let measurement = measure(
        args.workload,
        include_str!("../measure.py"),
        json!({
            "event_shape": args.event_shape,
            "exporter": args.exporter,
            "output": root,
            "library": library,
        }),
    )?;
    let discarded = discarded_events(
        args.exporter,
        root,
        measurement
            .context_id
            .as_deref()
            .ok_or("Python measurement omitted context ID")?
            .parse()?,
        args.workload,
    )?;
    let mut result = CaseResult::try_new(
        Framework::Quent,
        Language::Python,
        Some(args.exporter),
        Some(args.event_shape),
        args.workload,
        measurement.thread_batch_elapsed_ns,
        Some(discarded),
    )?;
    result.child_pid = measurement.child_pid;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
