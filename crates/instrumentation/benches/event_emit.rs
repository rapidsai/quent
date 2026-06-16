// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Microbenchmarks for the instrumentation API from the perspective of a
//! client emitting events.
//!
//! Single `emit` group with one entry per exporter backing (plus `noop`):
//! - `noop` — `Context::try_new(None)`; the cost a caller pays when
//!   instrumentation is compiled in but not active.
//! - `ndjson` / `msgpack` / `postcard` — write to a temp dir that is cleaned
//!   up when the bench function returns.
//! - `collector` — connect to an in-process gRPC server bound to a random
//!   localhost port, whose own backing exporter is ndjson into a temp dir.
//!
//! All exporters share the same caller-side hot path: `emit` builds an
//! `Event<T>` and pushes onto an unbounded mpsc. Serialization + I/O happen
//! asynchronously in the forwarder task, so the numbers reflect what
//! callers actually pay per event at the API boundary.

use std::path::Path;

use criterion::{BenchmarkGroup, Criterion, Throughput, black_box, measurement::WallTime};
use pprof::criterion::{Output, PProfProfiler};
use quent_collector::server::{CollectorService, CollectorServiceOptions};
use quent_collector_proto::collector_server::CollectorServer;
use quent_exporter::{
    CollectorExporterOptions, ExporterOptions, FileSystemExporterOptions, FileSystemFormat,
};
use quent_instrumentation::Context;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server as GrpcServer;
use uuid::Uuid;

type BenchResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize, Deserialize)]
struct BenchEvent;

impl quent_build_info::ModelSource for BenchEvent {
    fn package() -> &'static str {
        "quent-instrumentation"
    }
    fn source() -> quent_build_info::BuildInfo {
        quent_build_info::BuildInfo::unknown()
    }
}

// Starts the in-process collector server and leaks its runtime, so the
// server task and its file handles outlive `try_bench_emit`. Dropping the
// runtime mid-operation triggers an internal `unwrap` panic in tokio's
// async file path; leaking is fine because the bench process exits
// immediately after benches finish (OS reaps threads and FDs).
fn start_collector_server(backing_dir: &Path) -> BenchResult<String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .thread_name("bench-collector")
        .build()?;

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let address = format!("http://{}", std_listener.local_addr()?);
    std_listener.set_nonblocking(true)?;

    let backing = ExporterOptions::FileSystem(FileSystemExporterOptions {
        format: FileSystemFormat::Ndjson,
        root: backing_dir.to_path_buf(),
    });
    let opts = CollectorServiceOptions { exporter: backing };

    rt.spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(std_listener) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("bench collector: failed to adopt listener: {e}");
                return;
            }
        };
        let incoming = TcpListenerStream::new(listener);
        let service = CollectorService::<BenchEvent>::new(opts);
        let _ = GrpcServer::builder()
            .add_service(CollectorServer::new(service))
            .serve_with_incoming(incoming)
            .await;
    });

    std::mem::forget(rt);
    Ok(address)
}

fn bench_emit_variant(
    group: &mut BenchmarkGroup<'_, WallTime>,
    label: &str,
    exporter: Option<ExporterOptions>,
) -> BenchResult {
    let ctx = Context::<BenchEvent>::try_new(exporter)?;
    let sender = ctx.events_sender();
    let event_id = Uuid::now_v7();

    group.bench_function(label, |b| {
        b.iter(|| {
            sender.emit(black_box(event_id), black_box(BenchEvent));
        });
    });

    // Skip Drop: flushing buffered events through the forwarder on teardown
    // would dominate cleanup time and is not part of what we measure here.
    // The forwarder keeps its open file handles; writes continue to the
    // (eventually unlinked) inode until process exit.
    std::mem::forget(ctx);
    Ok(())
}

fn try_bench_emit(c: &mut Criterion) -> BenchResult {
    // TempDirs live until end of scope; each is removed from disk when its
    // binding drops here. Forgotten Contexts' forwarders keep their FDs
    // open across the unlink, so concurrent writes still succeed (Unix).
    let ndjson_dir = TempDir::new()?;
    let msgpack_dir = TempDir::new()?;
    let postcard_dir = TempDir::new()?;
    let collector_backing = TempDir::new()?;

    let collector_address = start_collector_server(collector_backing.path())?;

    let mut group = c.benchmark_group("emit");
    group.throughput(Throughput::Elements(1));

    bench_emit_variant(&mut group, "noop", None)?;
    bench_emit_variant(
        &mut group,
        "ndjson",
        Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Ndjson,
            root: ndjson_dir.path().to_path_buf(),
        })),
    )?;
    bench_emit_variant(
        &mut group,
        "msgpack",
        Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Msgpack,
            root: msgpack_dir.path().to_path_buf(),
        })),
    )?;
    bench_emit_variant(
        &mut group,
        "postcard",
        Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Postcard,
            root: postcard_dir.path().to_path_buf(),
        })),
    )?;
    bench_emit_variant(
        &mut group,
        "collector",
        Some(ExporterOptions::Collector(CollectorExporterOptions {
            address: collector_address,
            application_id: Uuid::now_v7(),
        })),
    )?;

    group.finish();
    Ok(())
}

fn bench_emit(c: &mut Criterion) {
    if let Err(e) = try_bench_emit(c) {
        eprintln!("bench setup failed: {e}");
        std::process::exit(1);
    }
}

// Knobs are env vars rather than CLI args because criterion owns argv parsing:
// its clap `Command` reads `std::env::args_os()` directly and rejects unknown
// flags. Injecting custom flags would require re-execing the process with a
// cleaned argv. Env vars sidestep that and let both knobs be set in one
// consistent way.
//
// `QUENT_BENCH_PROFILE_TIME` (seconds, per variant) triggers profile mode.
// `QUENT_BENCH_PROFILE_HZ` overrides the SIGPROF sampling rate (default 4999).
// Criterion's own `--profile-time` CLI flag still works.
fn build_criterion() -> Criterion {
    const DEFAULT_HZ: i32 = 4999;
    let hz: i32 = std::env::var("QUENT_BENCH_PROFILE_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_HZ);
    let criterion = Criterion::default()
        .with_profiler(PProfProfiler::new(hz, Output::Flamegraph(None)))
        .configure_from_args();
    // `configure_from_args` resets mode based on argv, so apply the env-var
    // profile-time override afterwards.
    if let Some(seconds) = std::env::var("QUENT_BENCH_PROFILE_TIME")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
    {
        criterion.profile_time(Some(std::time::Duration::from_secs_f64(seconds)))
    } else {
        criterion
    }
}

fn main() {
    let mut criterion = build_criterion();
    bench_emit(&mut criterion);
    criterion.final_summary();
}
