// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{num::NonZeroUsize, path::PathBuf, process::Command};

use clap::ValueEnum;

use crate::{
    BenchResult, SharedArgs,
    case::{CaseRunner, run_child},
    langs::rust,
    progress::BuildProgress,
    report::CaseResult,
};

/// Selects the exporter used by Quent benchmark cases.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Exporter {
    Noop,
    Ndjson,
    Msgpack,
    Postcard,
}

impl Exporter {
    /// Returns the exporter value accepted by the Quent implementation executable.
    fn as_str(self) -> &'static str {
        match self {
            Self::Noop => "noop",
            Self::Ndjson => "ndjson",
            Self::Msgpack => "msgpack",
            Self::Postcard => "postcard",
        }
    }
}

/// Selects the Quent-specific parameters.
#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct Args {
    #[arg(
        long = "quent-exporter",
        value_enum,
        value_delimiter = ',',
        default_value = "noop,ndjson,msgpack,postcard"
    )]
    exporter: Vec<Exporter>,
}

/// Defines one case for each requested exporter, event shape, and thread count.
pub(crate) fn cases(
    shared: &SharedArgs,
    options: &Args,
    progress: &mut BuildProgress,
) -> BenchResult<Vec<Box<dyn CaseRunner>>> {
    let executable = rust::binary("quent-bench-rust-quent", progress)?;
    let mut cases = Vec::new();
    for exporter in &options.exporter {
        for event_shape in &shared.event_shape {
            for threads in &shared.threads {
                cases.push(Box::new(QuentCase {
                    executable: executable.clone(),
                    exporter: *exporter,
                    event_shape: *event_shape,
                    threads: *threads,
                }) as Box<dyn CaseRunner>);
            }
        }
    }
    Ok(cases)
}

struct QuentCase {
    executable: PathBuf,
    exporter: Exporter,
    event_shape: quent_bench_types::EventShape,
    threads: NonZeroUsize,
}

impl CaseRunner for QuentCase {
    fn label(&self) -> String {
        format!(
            "quent, {}, {}, threads={}",
            self.exporter.as_str(),
            self.event_shape.as_ref(),
            self.threads
        )
    }

    fn run(&self, shared: &SharedArgs) -> BenchResult<CaseResult> {
        let mut command = Command::new(&self.executable);
        command
            .args(["--exporter", self.exporter.as_str()])
            .args(["--event-shape", self.event_shape.as_ref()]);
        shared.apply_workload(&mut command, self.threads);
        run_child(command)
    }
}
