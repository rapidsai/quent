// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_bench_types::MeasurementArgs;

pub type BenchResult<T> = Result<T, Box<dyn std::error::Error>>;

#[cxx::bridge(namespace = "quent_bench")]
pub mod ffi {
    struct Workload {
        threads: usize,
        num_batches: usize,
        batch_size: usize,
        num_warmup_batches: usize,
        batch_pause_us: u64,
        preflight_call: bool,
    }

    struct Measurement {
        elapsed_ns: Vec<u64>,
        context_id: String,
    }

    unsafe extern "C++" {
        include!("empty_loop.hpp");
        fn measure_empty(workload: &Workload) -> Result<Measurement>;
    }
}

impl TryFrom<MeasurementArgs> for ffi::Workload {
    type Error = Box<dyn std::error::Error>;

    fn try_from(args: MeasurementArgs) -> BenchResult<Self> {
        args.total_call_count()
            .ok_or("total call count overflows u64")?;
        let batches = args
            .batch
            .num_batches
            .get()
            .checked_add(args.batch.num_warmup_batches)
            .ok_or("batch count overflows usize")?;
        let batch_size = usize::try_from(args.batch.batch_size.get())?;
        batches
            .checked_mul(batch_size)
            .ok_or("payload count overflows usize")?;
        args.threads
            .get()
            .checked_mul(args.batch.num_batches.get())
            .ok_or("sample count overflows usize")?;
        Ok(Self {
            threads: args.threads.get(),
            num_batches: args.batch.num_batches.get(),
            batch_size,
            num_warmup_batches: args.batch.num_warmup_batches,
            batch_pause_us: args.batch.batch_pause_interval_us,
            preflight_call: !args.batch.no_preflight_call,
        })
    }
}

impl ffi::Measurement {
    pub fn thread_durations(&self, args: MeasurementArgs) -> BenchResult<Vec<Vec<u64>>> {
        let batches = args.batch.num_batches.get();
        if Some(self.elapsed_ns.len()) != args.threads.get().checked_mul(batches) {
            return Err("C++ measurement returned an unexpected sample count".into());
        }
        Ok(self
            .elapsed_ns
            .chunks_exact(batches)
            .map(<[u64]>::to_vec)
            .collect())
    }
}
