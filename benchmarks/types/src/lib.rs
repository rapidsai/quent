// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    num::{NonZeroU64, NonZeroUsize},
    process::Command,
};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Shared batch settings for benchmark command-line interfaces.
#[derive(clap::Args, Clone, Copy)]
pub struct BatchArgs {
    /// Number of measured batches per thread.
    #[arg(long, default_value = "1000")]
    pub num_batches: NonZeroUsize,
    /// Number of calls or empty-loop iterations per thread in each batch.
    #[arg(long, default_value = "20")]
    pub batch_size: NonZeroU64,
    /// Number of untimed batches per thread before measurement.
    #[arg(long, default_value_t = 10)]
    pub num_warmup_batches: usize,
    /// Minimum busy-wait between batches in microseconds (zero disables it).
    #[arg(long = "batch-pause-us", default_value_t = 10)]
    pub batch_pause_interval_us: u64,
    /// Skip the untimed preflight call made by each thread before its batches.
    #[arg(long)]
    pub no_preflight_call: bool,
}

impl BatchArgs {
    /// Appends the batch settings to a benchmark implementation command.
    pub fn append_to_command(&self, command: &mut Command) {
        command
            .args(["--num-batches", &self.num_batches.to_string()])
            .args(["--batch-size", &self.batch_size.to_string()])
            .args(["--num-warmup-batches", &self.num_warmup_batches.to_string()])
            .args([
                "--batch-pause-us",
                &self.batch_pause_interval_us.to_string(),
            ]);
        if self.no_preflight_call {
            command.arg("--no-preflight-call");
        }
    }
}

/// Measurement settings for one benchmark process and its concurrent caller threads.
#[derive(clap::Args, Clone, Copy)]
pub struct MeasurementArgs {
    #[arg(long)]
    pub threads: NonZeroUsize,
    #[command(flatten)]
    pub batch: BatchArgs,
}

impl MeasurementArgs {
    /// Appends this process's measurement settings to a benchmark implementation command.
    pub fn append_to_command(&self, command: &mut Command) {
        command.args(["--threads", &self.threads.to_string()]);
        self.batch.append_to_command(command);
    }

    /// Returns all attempted calls or empty-loop iterations, including warmup and optional preflight work.
    pub fn total_call_count(&self) -> Option<u64> {
        let batches = u64::try_from(self.batch.num_batches.get())
            .ok()?
            .checked_add(u64::try_from(self.batch.num_warmup_batches).ok()?)?;
        let calls_per_thread = batches
            .checked_mul(self.batch.batch_size.get())?
            .checked_add(u64::from(!self.batch.no_preflight_call))?;
        u64::try_from(self.threads.get())
            .ok()?
            .checked_mul(calls_per_thread)
    }
}

/// Identifies the implementation language of a benchmark case.
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    ValueEnum,
    strum::AsRefStr,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Language {
    Rust,
    Cpp,
    Python,
}

/// Identifies the framework or empty-loop baseline that produced a benchmark result.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, strum::AsRefStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Framework {
    EmptyLoopRs,
    EmptyLoopCpp,
    EmptyLoopPython,
    Quent,
}

/// Selects the explicit attributes of each benchmark event.
///
/// Payload index `i` advances from zero across batches, including warmup batches;
/// the optional preflight call uses `i = 0`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum, strum::AsRefStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum EventShape {
    /// Adds no explicit attributes.
    Empty,
    /// Adds `value: u8` set to `i mod 256`.
    U8,
    /// Adds `value: u64` set to `i`.
    U64,
    /// Adds `value: string` containing `"s"` repeated `8 + (i mod 9)` times.
    ShortString,
    /// Adds `value: string` containing `"l"` repeated `128 + (i mod 129)` times.
    LongString,
    /// Adds `small`, `large`, `short`, and `long` with the values of the four shapes above.
    All,
}

/// Records the settings and measured batch durations of one child-process case.
///
/// - `F`: Framework identifier type.
/// - `E`: Event exporting mechanism identifier type.
/// - `S`: Event shape identifier type.
#[derive(Debug, Deserialize, Serialize)]
pub struct CaseResult<F, E, S> {
    /// Names the benchmark framework or empty-loop baseline.
    #[serde(rename = "implementation")]
    pub framework: F,
    /// Language of the implementation executable.
    pub language: Language,
    /// Identifies how events are handled, or `None` when no output mechanism applies.
    pub exporter: Option<E>,
    /// Selects the event shape, or `None` when the case emits no event.
    pub event_shape: Option<S>,
    /// Number of threads making concurrent calls.
    pub threads: usize,
    /// Number of batches retained in the measurements.
    pub num_batches: usize,
    /// Number of calls or empty-loop iterations per thread in each batch.
    pub batch_size: u64,
    /// Number of initial batches excluded from the measurements.
    pub num_warmup_batches: usize,
    /// Minimum requested pause between batches, in microseconds.
    pub batch_pause_interval_us: u64,
    /// Whether each thread made one untimed call or empty-loop iteration before batching.
    pub preflight_call: bool,
    /// Process ID of the child that measured this case.
    pub child_pid: u32,
    /// Attempted calls or empty-loop iterations across all threads, including warmup and preflight.
    pub total_call_count: u64,
    /// Calls whose events were not retained; `None` for empty loops or unknown counts.
    pub discarded_call_count: Option<u64>,
    /// Elapsed nanoseconds for each measured batch, grouped by thread in spawn order.
    pub thread_batch_elapsed_ns: Vec<Vec<u64>>,
    /// Sum of measured batch durations divided by `threads * num_batches * batch_size`.
    pub average_ns_per_iteration: f64,
}

impl<F, E, S> CaseResult<F, E, S> {
    /// Builds a case result from the workload and measured batch durations.
    ///
    /// # Errors
    ///
    /// Returns an error if the total call count overflows `u64` or the discarded count exceeds it.
    pub fn try_new(
        framework: F,
        language: Language,
        exporter: Option<E>,
        event_shape: Option<S>,
        workload: MeasurementArgs,
        thread_batch_elapsed_ns: Vec<Vec<u64>>,
        discarded_call_count: Option<u64>,
    ) -> Result<Self, &'static str> {
        let total_call_count = workload
            .total_call_count()
            .ok_or("total call count overflows u64")?;
        if discarded_call_count.is_some_and(|count| count > total_call_count) {
            return Err("discarded call count exceeds total call count");
        }
        let average_ns_per_iteration = thread_batch_elapsed_ns
            .iter()
            .flatten()
            .map(|duration| *duration as f64)
            .sum::<f64>()
            / workload.threads.get() as f64
            / workload.batch.num_batches.get() as f64
            / workload.batch.batch_size.get() as f64;
        Ok(Self {
            framework,
            language,
            exporter,
            event_shape,
            threads: workload.threads.get(),
            num_batches: workload.batch.num_batches.get(),
            batch_size: workload.batch.batch_size.get(),
            num_warmup_batches: workload.batch.num_warmup_batches,
            batch_pause_interval_us: workload.batch.batch_pause_interval_us,
            preflight_call: !workload.batch.no_preflight_call,
            child_pid: std::process::id(),
            total_call_count,
            discarded_call_count,
            thread_batch_elapsed_ns,
            average_ns_per_iteration,
        })
    }
}
