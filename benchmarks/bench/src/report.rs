// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use chrono::Local;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, presets::UTF8_FULL};
use quent_bench_types::{CaseResult as SharedCaseResult, Framework};
use serde::Serialize;
use statrs::statistics::{Data, OrderStatistics, Statistics};

use crate::{BenchResult, EventShape, system::SystemProperties};

pub(crate) type CaseResult = SharedCaseResult<Framework, String, EventShape>;

pub(crate) fn print_system(system: &SystemProperties) {
    let mut table = Table::new();
    table.use_stderr();
    table.load_style(UTF8_FULL);
    table.set_header([header("Property"), header("Value")]);
    for (property, value) in [
        (
            "Captured (Unix seconds)",
            system.captured_at_unix_seconds.to_string(),
        ),
        ("OS", system.os.to_owned()),
        ("OS version", available(system.os_version.as_deref())),
        ("Kernel", available(system.kernel_version.as_deref())),
        ("Architecture", system.architecture.to_owned()),
        ("CPU model", available(system.cpu_model.as_deref())),
        ("Logical CPUs", available(system.logical_cpu_count)),
        ("Physical cores", available(system.physical_core_count)),
        ("Available CPUs", available(system.available_cpu_count)),
        ("RAM (bytes)", available(system.total_memory_bytes)),
        ("Rust compiler", available(system.rustc_version.as_deref())),
        ("Rust host", available(system.target_triple.as_deref())),
        ("Build profile", available(system.build_profile)),
        ("Git commit", available(system.git_commit.as_deref())),
        ("Git dirty", available(system.git_dirty)),
    ] {
        table.add_row([Cell::new(property).fg(Color::Cyan), Cell::new(value)]);
    }
    eprintln!("System properties:");
    eprintln!("{table}");
    eprintln!();
}

fn available(value: Option<impl std::fmt::Display>) -> String {
    value.map_or_else(|| "—".to_owned(), |value| value.to_string())
}

#[derive(Serialize)]
struct Report {
    system: SystemProperties,
    cases: Vec<ReportedCase>,
}

#[derive(Serialize)]
struct ReportedCase {
    #[serde(flatten)]
    result: CaseResult,
    batch_statistics: BatchStatistics,
}

#[derive(Serialize)]
struct BatchStatistics {
    standard_deviation_ns_per_iteration: f64,
    p50_ns_per_iteration: f64,
    p95_ns_per_iteration: f64,
    p99_ns_per_iteration: f64,
}

pub(crate) fn write(
    cases: Vec<CaseResult>,
    system: SystemProperties,
    output: Option<PathBuf>,
) -> BenchResult<()> {
    let report = Report {
        system,
        cases: cases
            .into_iter()
            .map(|result| ReportedCase {
                batch_statistics: batch_statistics(&result),
                result,
            })
            .collect(),
    };
    let output = match output {
        Some(path) => path,
        None => default_output(),
    };
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &output,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    print_cases(&report.cases);
    println!("{}", output.display());
    Ok(())
}

fn print_cases(cases: &[ReportedCase]) {
    if let Some(first) = cases.first() {
        let result = &first.result;
        let mut settings = Table::new();
        settings.use_stderr();
        settings.load_style(UTF8_FULL);
        settings.set_header([header("Setting"), header("Value")]);
        settings.add_row(vec![
            Cell::new("Measured batches").fg(Color::Cyan),
            Cell::new(result.num_batches),
        ]);
        settings.add_row(vec![
            Cell::new("Batch size").fg(Color::Cyan),
            Cell::new(format!("{} iterations/thread", result.batch_size)),
        ]);
        settings.add_row(vec![
            Cell::new("Warmup batches").fg(Color::Cyan),
            Cell::new(result.num_warmup_batches),
        ]);
        settings.add_row(vec![
            Cell::new("Requested spin pause").fg(Color::Cyan),
            Cell::new(format!("{} µs", result.batch_pause_interval_us)),
        ]);
        settings.add_row(vec![
            Cell::new("Preflight call").fg(Color::Cyan),
            Cell::new(if result.preflight_call {
                "enabled"
            } else {
                "disabled"
            }),
        ]);
        eprintln!("Run settings:");
        eprintln!("{settings}");
    }
    let mut table = Table::new();
    table.use_stderr();
    table.load_style(UTF8_FULL);
    table.set_header(
        [
            "Framework",
            "Language",
            "Exporter",
            "Event",
            "Threads",
            "Calls",
            "Discarded",
            "Mean",
            "SD",
            "p50",
            "p95",
            "p99",
        ]
        .map(header),
    );
    for case in cases {
        let result = &case.result;
        let stats = &case.batch_statistics;
        let row = vec![
            Cell::new(result.framework.as_ref()),
            Cell::new(result.language.as_ref()),
            Cell::new(result.exporter.as_deref().unwrap_or("—")),
            Cell::new(
                result
                    .event_shape
                    .as_ref()
                    .map(|shape| shape.as_ref())
                    .unwrap_or("—"),
            ),
            Cell::new(result.threads).set_alignment(CellAlignment::Right),
            Cell::new(result.total_call_count).set_alignment(CellAlignment::Right),
            Cell::new(available(result.discarded_call_count)).set_alignment(CellAlignment::Right),
            number(result.average_ns_per_iteration),
            number(stats.standard_deviation_ns_per_iteration),
            number(stats.p50_ns_per_iteration),
            number(stats.p95_ns_per_iteration),
            number(stats.p99_ns_per_iteration),
        ];
        table.add_row(
            row.into_iter()
                .map(|cell| cell.fg(framework_color(result.framework)))
                .collect::<Vec<_>>(),
        );
    }
    eprintln!();
    eprintln!("Batch averages (ns / single instrumentation call; empty loop: ns / iteration):");
    eprintln!("{table}");
}

fn header(label: &str) -> Cell {
    Cell::new(label)
        .fg(Color::Cyan)
        .add_attribute(Attribute::Bold)
}

fn framework_color(framework: Framework) -> Color {
    match framework {
        Framework::EmptyLoopRs | Framework::EmptyLoopCpp | Framework::EmptyLoopPython => {
            Color::Yellow
        }
        Framework::Quent => Color::Green,
    }
}

fn number(value: f64) -> Cell {
    Cell::new(format!("{value:.2}")).set_alignment(CellAlignment::Right)
}

fn batch_statistics(case: &CaseResult) -> BatchStatistics {
    let values = (0..case.num_batches)
        .map(|batch| {
            case.thread_batch_elapsed_ns
                .iter()
                .map(|thread| thread[batch] as f64)
                .sum::<f64>()
                / case.threads as f64
                / case.batch_size as f64
        })
        .collect::<Vec<_>>();
    let standard_deviation_ns_per_iteration = if values.len() > 1 {
        values.as_slice().std_dev()
    } else {
        0.0
    };
    let count = values.len();
    let mut values = Data::new(values);
    // Use order statistics to retain nearest-rank percentiles without interpolation.
    let mut percentile = |percent: usize| values.order_statistic((count * percent).div_ceil(100));
    BatchStatistics {
        standard_deviation_ns_per_iteration,
        p50_ns_per_iteration: percentile(50),
        p95_ns_per_iteration: percentile(95),
        p99_ns_per_iteration: percentile(99),
    }
}

fn default_output() -> PathBuf {
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("results");
    root.join(format!(
        "results-{timestamp}-pid{}.json",
        std::process::id()
    ))
}
