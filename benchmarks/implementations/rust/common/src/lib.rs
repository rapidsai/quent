// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    error::Error,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use quent_bench_types::MeasurementArgs;

pub type BenchResult<T> = Result<T, Box<dyn Error>>;

/// Measures concurrent calls in timed batches.
///
/// Creates one handle and thread per caller, then returns each thread's measured
/// batch durations in nanoseconds. Payload preparation, preflight calls, warmup
/// batches, and pauses are outside the timed intervals.
///
/// Infallible emitters can return `Ok::<(), std::convert::Infallible>(())`.
/// Monomorphization lets release builds remove the impossible error path from
/// their timed loop.
///
/// # Errors
///
/// Returns an emission error or an error if a duration does not fit in `u64`.
pub fn measure_threads<H, P, PrepareFn, EmitFn, E>(
    workload: MeasurementArgs,
    mut make_handle: impl FnMut() -> H,
    prepare: PrepareFn,
    emit: EmitFn,
) -> BenchResult<Vec<Vec<u64>>>
where
    H: Send + 'static,
    P: Send + 'static,
    PrepareFn: Fn(u64) -> P + Copy + Send + 'static,
    EmitFn: Fn(&H, P) -> Result<(), E> + Copy + Send + 'static,
    E: Error + Send + 'static,
{
    // Stage numbers follow the [measurement steps](../../../../README.md#measurement).
    let config = workload.batch;
    let threads = workload.threads.get();
    let preflight_call = !workload.batch.no_preflight_call;
    let total_batches = config.num_warmup_batches + config.num_batches.get();
    let barrier = Arc::new(Barrier::new(threads + 1));
    let failed = Arc::new(AtomicBool::new(false));
    let mut joins = Vec::with_capacity(threads);
    for _ in 0..threads {
        let handle = make_handle();
        let barrier = Arc::clone(&barrier);
        let failed = Arc::clone(&failed);
        joins.push(std::thread::spawn(move || {
            // Stage 1: Keep lazy first-call setup outside the measurements.
            let preflight_result = if preflight_call {
                emit(&handle, prepare(0))
            } else {
                Ok(())
            };
            if preflight_result.is_err() {
                failed.store(true, Ordering::SeqCst);
            }

            // Stage 2: Prepare all payloads before the first batch barrier.
            let batches = if preflight_result.is_ok() {
                (0..total_batches)
                    .map(|batch| {
                        let offset = (batch as u64).wrapping_mul(config.batch_size.get());
                        (0..config.batch_size.get())
                            .map(|index| prepare(offset.wrapping_add(index)))
                            .collect::<Vec<P>>()
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            barrier.wait();
            preflight_result?;
            if failed.load(Ordering::SeqCst) {
                return Ok(Vec::new());
            }
            let mut durations = Vec::with_capacity(config.num_batches.get());
            let mut batches = batches.into_iter();

            // Stage 3: Warmup, exercise the call path without recording timings.
            for payloads in batches.by_ref().take(config.num_warmup_batches) {
                barrier.wait();

                let batch_result = payloads
                    .into_iter()
                    .try_for_each(|payload| emit(&handle, payload));
                if batch_result.is_err() {
                    failed.store(true, Ordering::SeqCst);
                }
                barrier.wait();

                batch_result?;
                if failed.load(Ordering::SeqCst) {
                    return Ok(durations);
                }
                if config.batch_pause_interval_us > 0 {
                    busy_wait_pause(config.batch_pause_interval_us);
                }
            }

            // Stage 4: measured batches
            for (batch, payloads) in batches.enumerate() {
                barrier.wait();

                // Keep the batch buffer alive through the end timestamp so we
                // don't measure deallocation.
                let mut calls = payloads.into_iter();

                // Time the call loop with one clock read at each end.
                let start = Instant::now();
                // The actual measurement:
                // ----------------------------------------------------------------------
                let batch_result = calls
                    .by_ref()
                    .try_for_each(|payload| emit(&handle, payload));
                // ----------------------------------------------------------------------
                let elapsed = start.elapsed().as_nanos();

                drop(calls);
                if batch_result.is_err() {
                    failed.store(true, Ordering::SeqCst);
                }

                // Stage 5: Synchronize completion and errors before the optional pause.
                barrier.wait();
                batch_result?;
                durations.push(elapsed);
                if failed.load(Ordering::SeqCst) {
                    break;
                }
                if batch + 1 < config.num_batches.get() && config.batch_pause_interval_us > 0 {
                    busy_wait_pause(config.batch_pause_interval_us);
                }
            }
            Ok::<Vec<u128>, E>(durations)
        }));
    }
    barrier.wait();
    if !failed.load(Ordering::SeqCst) {
        for _ in 0..total_batches {
            barrier.wait();
            barrier.wait();
            if failed.load(Ordering::SeqCst) {
                break;
            }
        }
    }
    joins
        .into_iter()
        .map(|join| -> BenchResult<Vec<u64>> {
            join.join()
                .map_err(|_| "benchmark thread panicked")??
                .into_iter()
                .map(|elapsed| Ok(u64::try_from(elapsed)?))
                .collect()
        })
        .collect()
}

// Busy-wait instead of sleeping so threads stay active between instrumentation bursts.
fn busy_wait_pause(interval_us: u64) {
    let pause_started = Instant::now();
    let pause = Duration::from_micros(interval_us);
    while pause_started.elapsed() < pause {}
}
