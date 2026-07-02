// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The sync/async bridge across the runtime setups a client application can have:
//! a `#[tokio::main]` app, a current-thread app, a plain sync app (no runtime),
//! and an app with its own manually-managed runtime.

mod common;

use std::path::Path;

use common::{TestEvent, TestModel};
use quent_build_info::ModelSource;
use quent_exporter::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_instrumentation::{Context, Observer};
use uuid::Uuid;

fn fs_opts(root: &Path) -> ExporterOptions {
    ExporterOptions::FileSystem(FileSystemExporterOptions {
        format: FileSystemFormat::Ndjson,
        root: root.to_path_buf(),
    })
}

/// Build an observer through the one bridge, mirroring what a generated
/// `{App}Context::try_new` does (the sidecar is written at construction).
fn build(ctx: &Context) -> Observer<TestEvent> {
    ctx.block_on(ctx.observer::<TestEvent>()).unwrap()
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

/// Plain sync app, no ambient runtime: the context spawns its own (`Owned`), and
/// the bridge/drop block directly on it.
#[test]
fn plain_sync_app() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Context::try_new(TestModel::model_info(), Some(fs_opts(dir.path()))).unwrap();
    let id = ctx.id();
    {
        let observer = build(&ctx);
        observer.emit(Uuid::now_v7(), TestEvent);
    } // observer dropped on this non-runtime thread -> blocking flush
    assert_flushed(dir.path(), id);
}

/// `#[tokio::main]`-style app: an ambient multi-threaded runtime. The bridge and
/// the drop-time flush run on a worker via `block_in_place`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tokio_main_multi_thread() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Context::try_new(TestModel::model_info(), Some(fs_opts(dir.path()))).unwrap();
    let id = ctx.id();
    {
        let observer = build(&ctx);
        observer.emit(Uuid::now_v7(), TestEvent);
    } // observer dropped on a worker thread -> block_in_place flush
    assert_flushed(dir.path(), id);
}

/// `#[tokio::main(flavor = "current_thread")]` app: the bridge must block the only
/// worker, which is impossible — pins the documented panic so it can't silently
/// change.
#[tokio::test(flavor = "current_thread")]
#[should_panic]
async fn current_thread_runtime_panics() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Context::try_new(TestModel::model_info(), Some(fs_opts(dir.path()))).unwrap();
    let _ = build(&ctx);
}

/// App that builds and manages its own runtime and runs everything inside it.
#[test]
fn self_managed_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let id = rt.block_on(async {
        let ctx = Context::try_new(TestModel::model_info(), Some(fs_opts(dir.path()))).unwrap();
        let id = ctx.id();
        let observer = build(&ctx);
        observer.emit(Uuid::now_v7(), TestEvent);
        drop(observer); // flush on a worker of `rt`
        id
    });
    assert_flushed(dir.path(), id);
}

/// An `Owned`-runtime observer dropped from inside a *different* runtime: the last
/// `Arc` to the owned runtime is released on a worker thread. Must not panic (the
/// runtime shuts down without blocking) and must still flush.
#[test]
fn owned_runtime_observer_dropped_in_another_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Context::try_new(TestModel::model_info(), Some(fs_opts(dir.path()))).unwrap();
    let id = ctx.id();
    let observer = build(&ctx);
    observer.emit(Uuid::now_v7(), TestEvent);
    drop(ctx); // observer now solely keeps the owned runtime alive

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async move {
        drop(observer); // last owned-runtime Arc dropped on a worker of `rt`
    });
    assert_flushed(dir.path(), id);
}
