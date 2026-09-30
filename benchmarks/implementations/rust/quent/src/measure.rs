// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use quent_bench_rust_common::measure_threads;
use quent_bench_types::{EventShape, Implementation, Language, MeasurementArgs};
use quent_instrumentation::{
    Context, EventModel, ExporterOptions, FileSystemExporterOptions, HandleError,
    InstrumentedEntity, InstrumentedModel, Noop, ObserverBuilder, ObserverProvider,
    build_info::ModelSource,
};

use crate::{BenchResult, CaseResult, Exporter, models, verify};

pub fn run_case(
    exporter: Exporter,
    shape: EventShape,
    workload: MeasurementArgs,
) -> BenchResult<CaseResult> {
    let directory = exporter
        .file_format()
        .map(|_| tempfile::tempdir())
        .transpose()?;
    let export_root = directory.as_ref().map(|directory| directory.path());
    let (durations, discarded_call_count) = match shape {
        EventShape::Empty => {
            run_typed::<models::empty::LatencyEmpty, models::empty::Entity, _, _, _>(
                exporter,
                export_root,
                workload,
                |_| (),
                |handle, ()| handle.instr_call(),
            )?
        }
        EventShape::U8 => run_typed::<models::u8::LatencyU8, models::u8::Entity, _, _, _>(
            exporter,
            export_root,
            workload,
            |index| index as u8,
            |handle, value| handle.instr_call(value),
        )?,
        EventShape::U64 => run_typed::<models::u64::LatencyU64, models::u64::Entity, _, _, _>(
            exporter,
            export_root,
            workload,
            |index| index,
            |handle, value| handle.instr_call(value),
        )?,
        EventShape::ShortString => run_typed::<
            models::short_string::LatencyShortString,
            models::short_string::Entity,
            _,
            _,
            _,
        >(
            exporter,
            export_root,
            workload,
            short_string,
            |handle, value| handle.instr_call(value),
        )?,
        EventShape::LongString => {
            run_typed::<models::long_string::LatencyLongString, models::long_string::Entity, _, _, _>(
                exporter,
                export_root,
                workload,
                long_string,
                |handle, value| handle.instr_call(value),
            )?
        }
        EventShape::All => run_typed::<models::all::LatencyAll, models::all::Entity, _, _, _>(
            exporter,
            export_root,
            workload,
            |index| (index as u8, index, short_string(index), long_string(index)),
            |handle, (small, large, short, long)| handle.instr_call(small, large, short, long),
        )?,
    };
    Ok(CaseResult::try_new(
        Implementation::Quent,
        Language::Rust,
        Some(exporter),
        Some(shape),
        workload,
        durations,
        Some(discarded_call_count),
    )?)
}

fn short_string(index: u64) -> String {
    "s".repeat(8 + (index % 9) as usize)
}

fn long_string(index: u64) -> String {
    "l".repeat(128 + (index % 129) as usize)
}

fn run_typed<M, E, P, PrepareFn, EmitFn>(
    exporter: Exporter,
    export_root: Option<&Path>,
    workload: MeasurementArgs,
    prepare: PrepareFn,
    emit: EmitFn,
) -> BenchResult<(Vec<Vec<u64>>, u64)>
where
    M: EventModel
        + InstrumentedModel
        + ModelSource
        + ObserverBuilder<Noop>
        + ObserverBuilder<ExporterOptions>,
    M::Observers: ObserverProvider<E>,
    E: InstrumentedEntity<Context = Context<M>>,
    E::Handle: Send + 'static,
    P: Send + 'static,
    PrepareFn: Fn(u64) -> P + Copy + Send + 'static,
    EmitFn: Fn(&E::Handle, P) -> Result<(), HandleError> + Copy + Send + 'static,
{
    let context = match exporter.file_format() {
        None => Context::<M>::try_new(Noop)?,
        Some(format) => {
            Context::<M>::try_new(ExporterOptions::FileSystem(FileSystemExporterOptions::new(
                format,
                export_root
                    .ok_or("filesystem export root is missing")?
                    .to_path_buf(),
            )))?
        }
    };
    let context_id = context.id();
    let observer = context.observer::<E>();
    let durations = measure_threads(workload, || observer.handle(), prepare, emit)?;
    drop(observer);
    drop(context);

    let discarded_call_count =
        verify::discarded_events(exporter, export_root, context_id, workload)?;
    Ok((durations, discarded_call_count))
}
