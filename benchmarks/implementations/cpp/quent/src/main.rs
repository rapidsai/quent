// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, ValueEnum};
use quent_bench_cpp_common::ffi as common;
use quent_bench_quent_common::{BenchResult, Exporter, discarded_events};
use quent_bench_types::{CaseResult, EventShape, Framework, Language, MeasurementArgs};

#[allow(unused)]
mod models {
    include!(concat!(env!("OUT_DIR"), "/models.rs"));
}

#[cxx::bridge(namespace = "quent_bench")]
mod ffi {
    unsafe extern "C++" {
        include!("quent-bench-cpp-common/src/lib.rs.h");
        include!("quent.hpp");
        type Workload = quent_bench_cpp_common::ffi::Workload;
        type Measurement = quent_bench_cpp_common::ffi::Measurement;
        fn measure_quent(
            workload: &Workload,
            shape: &str,
            exporter: &str,
            output: &str,
        ) -> Result<Measurement>;
    }
}

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
    let workload = common::Workload::try_from(args.workload)?;
    let directory = args
        .exporter
        .file_format()
        .map(|_| tempfile::tempdir())
        .transpose()?;
    let root = directory.as_ref().map(|directory| directory.path());
    let output = root
        .map(|path| path.to_str().ok_or("export path is not UTF-8"))
        .transpose()?
        .unwrap_or("");
    let measurement = ffi::measure_quent(
        &workload,
        args.event_shape.as_ref(),
        args.exporter.to_possible_value().unwrap().get_name(),
        output,
    )?;
    let discarded = discarded_events(
        args.exporter,
        root,
        measurement.context_id.parse()?,
        args.workload,
    )?;
    let result = CaseResult::try_new(
        Framework::Quent,
        Language::Cpp,
        Some(args.exporter),
        Some(args.event_shape),
        args.workload,
        measurement.thread_durations(args.workload)?,
        Some(discarded),
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
