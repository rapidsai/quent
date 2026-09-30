// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

mod measure;
mod models;
mod verify;

use clap::{Parser, ValueEnum};
use quent_bench_rust_common::BenchResult;
use quent_bench_types::{
    CaseResult as SharedCaseResult, EventShape, Implementation, MeasurementArgs,
};
use quent_instrumentation::FileSystemFormat;
use serde::Serialize;

type CaseResult = SharedCaseResult<Implementation, Exporter, EventShape>;

#[derive(Clone, Copy, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Exporter {
    Noop,
    Ndjson,
    Msgpack,
    Postcard,
}

impl Exporter {
    fn file_format(self) -> Option<FileSystemFormat> {
        match self {
            Self::Noop => None,
            Self::Ndjson => Some(FileSystemFormat::Ndjson),
            Self::Msgpack => Some(FileSystemFormat::Msgpack),
            Self::Postcard => Some(FileSystemFormat::Postcard),
        }
    }
}

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
