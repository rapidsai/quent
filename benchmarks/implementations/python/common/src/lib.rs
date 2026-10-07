// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

use quent_bench_types::MeasurementArgs;
use serde::Deserialize;
use serde_json::{Value, json};

pub type BenchResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
pub struct Measurement {
    pub thread_batch_elapsed_ns: Vec<Vec<u64>>,
    pub child_pid: u32,
    pub context_id: Option<String>,
}

/// Runs a Python measurement in a separate interpreter process.
///
/// Uses `PYO3_PYTHON` when set, otherwise `python3`. The script receives `config`
/// and the shared `measure_threads` function and must print one JSON measurement.
pub fn measure(
    workload: MeasurementArgs,
    script: &str,
    options: Value,
) -> BenchResult<Measurement> {
    workload
        .total_call_count()
        .ok_or("total call count overflows u64")?;
    workload
        .batch
        .num_batches
        .get()
        .checked_add(workload.batch.num_warmup_batches)
        .ok_or("batch count overflows usize")?;
    let config = json!({
        "threads": workload.threads.get(),
        "num_batches": workload.batch.num_batches.get(),
        "batch_size": workload.batch.batch_size.get(),
        "num_warmup_batches": workload.batch.num_warmup_batches,
        "batch_pause_us": workload.batch.batch_pause_interval_us,
        "preflight_call": !workload.batch.no_preflight_call,
        "options": options,
    });
    let source = format!("{}\n{script}", include_str!("../measure.py"));
    let python = std::env::var_os("PYO3_PYTHON").unwrap_or_else(|| "python3".into());
    let output = Command::new(python)
        .args(["-c", &source, &config.to_string()])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "Python measurement failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let measurement: Measurement = serde_json::from_slice(&output.stdout)?;
    if measurement.thread_batch_elapsed_ns.len() != workload.threads.get()
        || measurement
            .thread_batch_elapsed_ns
            .iter()
            .any(|thread| thread.len() != workload.batch.num_batches.get())
    {
        return Err("Python measurement returned an unexpected sample count".into());
    }
    Ok(measurement)
}
