// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Emits a small generated instrumentation model through selectable exporters.
//! Native thread IDs are supported on Linux, macOS, and Windows.

use std::path::PathBuf;

use quent_instrumentation::{
    EventCallback, ExporterOptions, FileSystemExporterOptions, FileSystemFormat,
};

use demo::{
    Connection, Context, Demo, DemoEvent, Handle, Observer, Query, Server, Thread, ThreadPool,
    ThreadUsage, Uuid,
};

#[allow(unused)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

/// Emits the demo events through a debug-printing callback.
pub fn run_with_debug_print() -> Result<Uuid, Box<dyn std::error::Error>> {
    let context = Context::try_new(EventCallback::<DemoEvent>::new(|event| {
        println!("{event:?}")
    }))?;
    emit_events(context)
}

/// Exports the demo events as NDJSON and returns their context ID.
pub fn run_with_ndjson(
    root_export_path: impl Into<PathBuf>,
) -> Result<Uuid, Box<dyn std::error::Error>> {
    let context = Context::try_new(ExporterOptions::FileSystem(FileSystemExporterOptions::new(
        FileSystemFormat::Ndjson,
        root_export_path.into(),
    )))?;
    emit_events(context)
}

fn emit_events(context: Context<Demo>) -> Result<Uuid, Box<dyn std::error::Error>> {
    // The context builds one exporter pipeline per entity event type and
    // exposes the corresponding typed observers.
    let context_id = context.id();

    // `observer.handle()` creates a fresh entity instance to emit events for.
    let mut server = context.observer::<Server>().handle();
    server.booted(demo::quent::os::Process {
        native_id: std::process::id(),
    })?;

    let mut pool = context.observer::<ThreadPool>().handle();
    pool.created(server.as_entity_ref())?;

    let mut thread = context.observer::<Thread>().handle();
    thread.started(
        demo::quent::os::Thread {
            native_id: current_native_thread_id()?,
        },
        pool.as_entity_ref(),
    )?;

    let observer: Observer<Connection> = context.observer::<Connection>();
    // Once-cardinality events take `&mut self` and may fire only once, tracked
    // by the handle, hence it is mut:
    let mut conn: Handle<Connection> = observer.handle();

    // One method per entity event:
    conn.opened(
        demo::Endpoint {
            host: "localhost".to_owned(),
            port: 8080,
        },
        Uuid::nil(),
        // A handle can deal out a reference to the entity it represents:
        server.as_entity_ref(),
    )?;
    conn.data(1234, None)?;

    // A `dynamic` schema field maps to `DynamicAttributes`, which are
    // dynamically-typed key-value pairs:
    let mut extra = demo::DynamicAttributes::new();
    extra.add_string("peer_agent", "curl/8.4");
    extra.add_u64("chunk_index", 3);
    extra.add_bool("compressed", true);
    conn.data(
        5678,
        Some(demo::Meta {
            tags: vec!["tls".to_string(), "keepalive".to_string()],
            extra,
        }),
    )?;

    // `as_entity_ref_with` produces an entity ref that also carries data:
    conn.routed(server.as_entity_ref_with(demo::Route { hops: 3 }))?;

    // FSM transitions consume the prior handle and return the target state's
    // handle.
    let query = context
        .observer::<Query>()
        .handle()
        .submitted("select 1".to_owned(), conn.as_entity_ref())
        .running(10, thread.as_entity_ref_with(ThreadUsage))
        .running(20, thread.as_entity_ref_with(ThreadUsage))
        .ready(true);
    let _query_id = query.uuid();

    conn.closed()?;

    // A once-event returns an error if emitted again.
    assert!(conn.closed_emitted());
    assert!(conn.closed().is_err());

    Ok(context_id)
}

#[cfg(target_os = "linux")]
fn current_native_thread_id() -> std::io::Result<u64> {
    // SAFETY: `gettid` takes no arguments and returns the caller's kernel task ID.
    let native_id = unsafe { libc::syscall(libc::SYS_gettid) };
    if native_id < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(native_id as u64)
    }
}

#[cfg(target_os = "macos")]
fn current_native_thread_id() -> std::io::Result<u64> {
    let mut native_id = 0;
    // SAFETY: A null thread selects the calling thread, and `native_id` is writable.
    let result = unsafe { libc::pthread_threadid_np(0, &mut native_id) };
    if result == 0 {
        Ok(native_id)
    } else {
        Err(std::io::Error::from_raw_os_error(result))
    }
}

#[cfg(windows)]
fn current_native_thread_id() -> std::io::Result<u64> {
    // SAFETY: `GetCurrentThreadId` has no preconditions.
    Ok(unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }.into())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn current_native_thread_id() -> std::io::Result<u64> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "native thread IDs are unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn native_thread_id_is_nonzero() {
        assert_ne!(super::current_native_thread_id().unwrap(), 0);
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    #[test]
    fn native_thread_id_is_unsupported() {
        assert_eq!(
            super::current_native_thread_id().unwrap_err().kind(),
            std::io::ErrorKind::Unsupported
        );
    }
}
