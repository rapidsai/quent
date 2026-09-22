// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Emits a small generated instrumentation model through selectable exporters.
//! Native thread IDs are supported on Linux, macOS, and Windows.

use std::path::PathBuf;

use nvtx_events::{NvtxColor, NvtxEventAttributes, NvtxMessage, NvtxPayload, NvtxPayloadValue};
use quent_instrumentation::{
    EventCallback, ExporterOptions, FileSystemExporterOptions, FileSystemFormat,
};

use demo::{
    Connection, Context, Demo, DemoEvent, Handle, NvtxEvent, Observer, Query, Server, Thread,
    ThreadPool, ThreadUsage, Uuid,
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

    // NVTX is a private generated stream bound to this process entity. Native
    // callback data converts directly into the schema event vocabulary and
    // uses the same observer/exporter pipeline as every other event.
    let mut nvtx = context.observer::<NvtxEvent>().handle();
    nvtx.initialized(server.as_entity_ref())?;
    let native_thread_id = current_native_thread_id()?;
    let nvtx_thread_id = u32::try_from(native_thread_id)?;
    nvtx.name_thread(nvtx_thread_id, "demo-main".to_owned())?;
    nvtx.range_start(
        0,
        1,
        NvtxEventAttributes {
            category: u32::MAX,
            color: Some(NvtxColor {
                color_type: i32::MIN,
                value: u32::MAX,
            }),
            message: Some(NvtxMessage::String("generated-shared-io".to_owned())),
            payload: Some(NvtxPayload {
                payload_type: i32::MAX,
                value: NvtxPayloadValue::Double(f64::from_bits(0x7ff8_0000_0000_1234)),
            }),
        }
        .into(),
    )?;
    nvtx.range_end(0, 1)?;

    let mut pool = context.observer::<ThreadPool>().handle();
    pool.created(server.as_entity_ref())?;

    let mut thread = context.observer::<Thread>().handle();
    thread.started(
        demo::quent::os::Thread {
            native_id: native_thread_id,
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
    extra.add("peer_agent", "curl/8.4");
    extra.add("chunk_index", 3_u64);
    extra.add("compressed", true);
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
    #[cfg(feature = "nvtx-capture")]
    use std::sync::{Arc, Mutex};

    use nvtx_analyzer::NvtxEventData;
    use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage, NvtxPayload, NvtxPayloadValue};

    use crate::demo::NvtxEventEvent;

    #[cfg(feature = "nvtx-capture")]
    use crate::demo::{
        Context, ContextOptions, Demo, DemoEvent, HandleError, Noop, Server, ServerEvent,
        SourceCapture,
    };

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

    #[test]
    fn generated_nvtx_conversion_covers_the_native_vocabulary() {
        let attributes = || NvtxEventAttributes {
            category: 17,
            message: Some(NvtxMessage::RegisteredHandle(19)),
            payload: Some(NvtxPayload {
                payload_type: -23,
                value: NvtxPayloadValue::Int64(i64::MIN),
            }),
            ..Default::default()
        };
        let native = [
            NvtxEvent::RangePush {
                domain: 1,
                thread_id: 2,
                attributes: attributes(),
            },
            NvtxEvent::RangePop {
                domain: 1,
                thread_id: 2,
            },
            NvtxEvent::RangeStart {
                domain: 1,
                range_id: 3,
                attributes: attributes(),
            },
            NvtxEvent::RangeEnd {
                domain: 1,
                range_id: 3,
            },
            NvtxEvent::Mark {
                domain: 1,
                attributes: attributes(),
            },
            NvtxEvent::DomainCreate {
                domain: 1,
                name: "domain".to_owned(),
            },
            NvtxEvent::DomainDestroy { domain: 1 },
            NvtxEvent::RegisterString {
                domain: 1,
                handle: 19,
                string: "registered".to_owned(),
            },
            NvtxEvent::NameCategory {
                domain: 1,
                category: 17,
                name: "category".to_owned(),
            },
            NvtxEvent::NameThread {
                thread_id: 2,
                name: "thread".to_owned(),
            },
            NvtxEvent::ResourceCreate {
                domain: 1,
                handle: 5,
                identifier_type: i32::MIN,
                identifier: u64::MAX,
                message: Some(NvtxMessage::String("resource".to_owned())),
            },
            NvtxEvent::ResourceDestroy { handle: 5 },
        ];

        for event in native {
            let generated: NvtxEventEvent = event.clone().into();
            assert_eq!(generated.nvtx_event(), event.nvtx_event());
        }
    }

    #[cfg(feature = "nvtx-capture")]
    #[test]
    fn process_identity_controls_private_live_capture() {
        // No-op contexts never retain the generated activation callback and
        // therefore accept synthetic process identities without attaching.
        let noop = Context::<Demo>::try_new(Noop).unwrap();
        let mut noop_server = noop.observer::<Server>().handle();
        noop_server
            .booted(crate::demo::quent::os::Process {
                native_id: u32::MAX,
            })
            .unwrap();
        drop(noop);

        // An active context can explicitly disable source capture while still
        // exporting ordinary process events.
        let disabled_events = Arc::new(Mutex::new(Vec::new()));
        let disabled = Context::<Demo>::try_new_with_options(
            quent_instrumentation::EventCallback::<DemoEvent>::new({
                let events = Arc::clone(&disabled_events);
                move |event| events.lock().unwrap().push(event)
            }),
            ContextOptions::default().with_source_capture(SourceCapture::Disabled),
        )
        .unwrap();
        let mut disabled_server = disabled.observer::<Server>().handle();
        disabled_server
            .booted(crate::demo::quent::os::Process {
                native_id: u32::MAX,
            })
            .unwrap();
        drop(disabled_server);
        drop(disabled);
        let disabled_events = disabled_events.lock().unwrap();
        assert!(
            disabled_events
                .iter()
                .any(|event| matches!(&event.data, DemoEvent::Server(ServerEvent::Booted { .. })))
        );
        assert!(
            disabled_events
                .iter()
                .all(|event| !matches!(&event.data, DemoEvent::NvtxEvent(_)))
        );
        drop(disabled_events);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let context =
            Context::<Demo>::try_new(quent_instrumentation::EventCallback::<DemoEvent>::new({
                let captured = Arc::clone(&captured);
                move |event| captured.lock().unwrap().push(event)
            }))
            .unwrap();
        let mut server = context.observer::<Server>().handle();
        let process_id = server.uuid();

        // A foreign PID fails before the once flag or either binding event is
        // emitted, so the caller can retry with this process's real PID.
        let foreign_pid = std::process::id().wrapping_add(1);
        assert!(matches!(
            server.booted(crate::demo::quent::os::Process {
                native_id: foreign_pid,
            }),
            Err(HandleError::SourceActivation { .. })
        ));
        assert!(!server.booted_emitted());
        server
            .booted(crate::demo::quent::os::Process {
                native_id: std::process::id(),
            })
            .unwrap();
        nvtx::mark(c"generated-private-capture");
        drop(server);
        drop(context);

        let captured = captured.lock().unwrap();
        assert!(captured.iter().any(|event| {
            event.id == process_id
                && matches!(&event.data, DemoEvent::Server(ServerEvent::Booted { .. }))
        }));
        let nvtx = captured
            .iter()
            .filter_map(|event| match &event.data {
                DemoEvent::NvtxEvent(data) => Some((event.id, data)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            nvtx.first(),
            Some((_, NvtxEventEvent::Initialized { process })) if process.target == process_id
        ));
        assert!(nvtx.iter().any(|(_, event)| matches!(
            event,
            NvtxEventEvent::Mark { attributes, .. }
                if attributes.message.as_ref().and_then(|message| message.string.as_deref())
                    == Some("generated-private-capture")
        )));
        drop(captured);

        // The current backend is process-global and one-shot until #696. A
        // second active context receives the installation failure as a typed
        // handle error rather than panicking or exporting a phantom private
        // binding.
        let failed = Arc::new(Mutex::new(Vec::new()));
        let second =
            Context::<Demo>::try_new(quent_instrumentation::EventCallback::<DemoEvent>::new({
                let failed = Arc::clone(&failed);
                move |event| failed.lock().unwrap().push(event)
            }))
            .unwrap();
        let mut second_server = second.observer::<Server>().handle();
        assert!(matches!(
            second_server.booted(crate::demo::quent::os::Process {
                native_id: std::process::id(),
            }),
            Err(HandleError::SourceActivation { .. })
        ));
        drop(second_server);
        drop(second);
        assert!(
            failed
                .lock()
                .unwrap()
                .iter()
                .all(|event| !matches!(&event.data, DemoEvent::NvtxEvent(_)))
        );
    }
}
