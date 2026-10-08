// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Test synchronous bridging, runtime ownership, and observer lifetime.

mod common;

use std::path::Path;

use common::TestEvent;
use quent_instrumentation::{ContextInner, ObserverInner};
use quent_io::ExporterOptions;
use quent_io::filesystem::{self, Format};
use uuid::Uuid;

fn fs_opts(root: &Path) -> ExporterOptions {
    ExporterOptions::FileSystem(filesystem::exporter::Options::new(
        Format::Ndjson,
        root.to_path_buf(),
    ))
}

/// Creates an active context and filesystem exporter options for `root`.
fn active(root: &Path) -> (ContextInner, ExporterOptions, Uuid) {
    let id = Uuid::now_v7();
    let ctx = ContextInner::try_new(id).unwrap();
    let exporter_opts = fs_opts(root);
    (ctx, exporter_opts, id)
}

/// Creates an observer using the context's synchronous bridge.
fn build(ctx: &ContextInner, exporter_opts: &ExporterOptions) -> ObserverInner<TestEvent> {
    ctx.block_on(async { ctx.observer::<TestEvent>(exporter_opts).await })
        .unwrap()
}

/// Assert the observer flushed one non-empty ndjson batch under `<root>/<id>/`.
fn assert_flushed(root: &Path, id: Uuid) {
    let entity_dir = root.join(id.to_string()).join("TestEvent");
    let files: Vec<_> = std::fs::read_dir(&entity_dir)
        .unwrap_or_else(|e| panic!("reading {entity_dir:?}: {e}"))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("ndjson"))
        .collect();
    assert_eq!(
        files.len(),
        1,
        "expected one ndjson batch in {entity_dir:?}"
    );
    assert!(
        files[0].metadata().unwrap().len() > 0,
        "ndjson batch is empty"
    );
}

/// Construction and drop-time flushing work without an ambient runtime.
#[test]
fn plain_sync_app() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, exporter_opts, id) = active(dir.path());
    {
        let observer = build(&ctx, &exporter_opts);
        observer.emit(Uuid::now_v7(), TestEvent);
    }
    assert_flushed(dir.path(), id);
}

/// Construction and drop-time flushing work inside a multi-threaded runtime.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tokio_main_multi_thread() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, exporter_opts, id) = active(dir.path());
    {
        let observer = build(&ctx, &exporter_opts);
        observer.emit(Uuid::now_v7(), TestEvent);
    }
    assert_flushed(dir.path(), id);
}

/// The synchronous bridge retains its documented current-thread runtime panic.
#[tokio::test(flavor = "current_thread")]
#[should_panic(expected = "can call blocking only when running on the multi-threaded runtime")]
async fn current_thread_runtime_panics() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, exporter_opts, _id) = active(dir.path());
    let _ = build(&ctx, &exporter_opts);
}

/// Dropping the final runtime owner inside an async context flushes without panicking.
#[test]
fn owned_runtime_observer_dropped_in_another_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, exporter_opts, id) = active(dir.path());
    let observer = build(&ctx, &exporter_opts);
    observer.emit(Uuid::now_v7(), TestEvent);
    // The observer is now the final owner of its runtime.
    drop(ctx);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async move {
        drop(observer);
    });
    assert_flushed(dir.path(), id);
}

/// The observer is created inside an application-owned runtime but keeps its own
/// runtime alive after both the application runtime and its context are dropped.
/// Events emitted afterward are still flushed on observer drop.
#[test]
fn observer_outlives_ambient_runtime_and_context() {
    let dir = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let (ctx, observer, id) = rt.block_on(async {
        let (ctx, exporter_opts, id) = active(dir.path());
        let observer = build(&ctx, &exporter_opts);
        (ctx, observer, id)
    });
    drop(rt);
    drop(ctx);
    observer.emit(Uuid::now_v7(), TestEvent);
    drop(observer);
    assert_flushed(dir.path(), id);
}
