// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

mod case;
mod frameworks;
mod langs;
mod progress;
mod report;
mod system;

use std::{collections::BTreeSet, num::NonZeroUsize, path::PathBuf, process::Command};

use clap::{Parser, ValueEnum};
use quent_bench_types::{BatchArgs, EventShape, Language, MeasurementArgs};
use serde::{Deserialize, Serialize};

use case::CaseRunner;
use progress::{BuildProgress, ProgressLine};

type BenchResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Selects an instrumentation framework for benchmark cases.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Framework {
    Quent,
}

impl Framework {
    /// Identifies the language used to deduplicate empty-loop cases across frameworks.
    fn language(self) -> Language {
        match self {
            Self::Quent => Language::Rust,
        }
    }
}

/// Combines benchmark selection, shared workload settings, and framework-specific options.
#[derive(Parser)]
#[command(about = "Measure generated instrumentation calls in isolated processes")]
struct Args {
    /// Frameworks to benchmark.
    #[arg(long, value_enum, value_delimiter = ',', default_value = "quent")]
    frameworks: Vec<Framework>,
    /// Whether to measure an empty loop for each selected language.
    #[arg(long)]
    empty_loop: bool,
    /// Path for the JSON report.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Workload settings for the run.
    #[command(flatten)]
    shared: SharedArgs,
    /// Exporter settings applied only to Quent cases.
    #[command(flatten)]
    quent: frameworks::quent::Args,
}

/// Stores cross-framework workload settings.
#[derive(clap::Args)]
struct SharedArgs {
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        default_value = "empty,u8,u64,short-string,long-string,all"
    )]
    event_shape: Vec<EventShape>,
    #[arg(long, value_delimiter = ',', default_value = "1")]
    threads: Vec<NonZeroUsize>,
    #[command(flatten)]
    batch: BatchArgs,
}

impl SharedArgs {
    fn apply_workload(&self, command: &mut Command, threads: NonZeroUsize) {
        MeasurementArgs {
            threads,
            batch: self.batch,
        }
        .append_to_command(command);
    }
}

impl Args {
    fn case_runners(&self) -> BenchResult<Vec<Box<dyn CaseRunner>>> {
        let mut runners = Vec::new();
        let languages = self
            .frameworks
            .iter()
            .map(|framework| framework.language())
            .collect::<BTreeSet<_>>();
        let mut builds = BuildProgress::new(
            self.frameworks.len() + if self.empty_loop { languages.len() } else { 0 },
        );
        if self.empty_loop {
            for language in languages {
                match language {
                    Language::Rust => {
                        runners.extend(langs::rust::empty_loop_cases(&self.shared, &mut builds)?);
                    }
                }
            }
        }
        for framework in self.frameworks.iter().copied() {
            match framework {
                Framework::Quent => {
                    runners.extend(frameworks::quent::cases(
                        &self.shared,
                        &self.quent,
                        &mut builds,
                    )?);
                }
            }
        }
        builds.finish();
        Ok(runners)
    }
}

fn main() -> BenchResult<()> {
    let args = Args::parse();
    let shared = &args.shared;
    let system = system::properties()?;
    let runners = args.case_runners()?;
    let total = runners.len();
    let mut cases = Vec::with_capacity(total);
    let mut progress = ProgressLine::new();
    for (index, runner) in runners.into_iter().enumerate() {
        progress.update(&format!(
            "Running case ({}/{total}): {}",
            index + 1,
            runner.label()
        ));
        cases.push(runner.run(shared)?);
    }
    progress.update(&format!(
        "Running case ({total}/{total}): all runs completed"
    ));
    progress.finish();

    report::print_system(&system);
    report::write(cases, system, args.output)
}
