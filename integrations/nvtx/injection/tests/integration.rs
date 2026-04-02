// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Integration test: calls NVTX API via nvidia-nvtx, verifies events arrive
//! through the injection with correct types and data.
//!
//! This is a single test because NVTX initialization is process-global and
//! one-shot — the injection can only be installed once per process.

use std::sync::{Arc, Mutex};

use quent_nvtx_events::{NvtxEvent, NvtxMessage};

#[test]
fn nvtx_injection_captures_events() {
    let events: Arc<Mutex<Vec<NvtxEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);

    // Install MUST happen before any NVTX call.
    quent_nvtx_injection::install_hook(move |event| {
        events_clone.lock().unwrap().push(event);
    });

    // --- Mark (default domain) ---
    nvtx::mark("test mark");

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1, "expected 1 event after mark");
        match &captured[0] {
            NvtxEvent::Mark(m) => {
                assert!(m.thread_id > 0);
                assert!(m.domain_handle_id.is_none());
                let msg = m.attributes.as_ref().unwrap().message.as_ref().unwrap();
                assert_eq!(msg, &NvtxMessage::String("test mark".into()));
            }
            other => panic!("expected Mark, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    // --- Push/Pop (LocalRange, default domain) ---
    {
        let _range = nvtx::LocalRange::new("my range");
    }

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 2, "expected 2 events after push/pop");
        match &captured[0] {
            NvtxEvent::Push(p) => {
                assert!(p.thread_id > 0);
                assert!(p.domain_handle_id.is_none());
                let msg = p.attributes.as_ref().unwrap().message.as_ref().unwrap();
                assert_eq!(msg, &NvtxMessage::String("my range".into()));
            }
            other => panic!("expected Push, got {other:?}"),
        }
        match &captured[1] {
            NvtxEvent::Pop(p) => {
                assert!(p.thread_id > 0);
                assert!(p.domain_handle_id.is_none());
            }
            other => panic!("expected Pop, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    // --- Start/End (Range, default domain) ---
    {
        let _range = nvtx::Range::new("process range");
    }

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 2, "expected 2 events after start/end");
        let start_id = match &captured[0] {
            NvtxEvent::RangeStart(r) => {
                assert!(r.domain_handle_id.is_none());
                let msg = r.attributes.as_ref().unwrap().message.as_ref().unwrap();
                assert_eq!(msg, &NvtxMessage::String("process range".into()));
                r.range_handle_id
            }
            other => panic!("expected RangeStart, got {other:?}"),
        };
        match &captured[1] {
            NvtxEvent::RangeEnd(r) => {
                assert_eq!(r.range_handle_id, start_id);
                assert!(r.domain_handle_id.is_none());
            }
            other => panic!("expected RangeEnd, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    // --- Nested push/pop ---
    {
        let _outer = nvtx::LocalRange::new("outer");
        {
            let _inner = nvtx::LocalRange::new("inner");
        }
    }

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 4, "expected 4 events for nested push/pop");
        assert!(matches!(&captured[0], NvtxEvent::Push(_)));
        assert!(matches!(&captured[1], NvtxEvent::Push(_)));
        assert!(matches!(&captured[2], NvtxEvent::Pop(_)));
        assert!(matches!(&captured[3], NvtxEvent::Pop(_)));
    }
    events.lock().unwrap().clear();

    // --- Domain creation + domain-scoped mark ---
    let domain = nvtx::Domain::new("test-domain");

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1, "expected 1 DomainCreate event");
        match &captured[0] {
            NvtxEvent::DomainCreate(d) => {
                assert!(d.domain_handle_id > 0);
                assert_eq!(d.name, "test-domain");
            }
            other => panic!("expected DomainCreate, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    domain.mark("domain mark");

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1, "expected 1 domain Mark event");
        match &captured[0] {
            NvtxEvent::Mark(m) => {
                assert!(m.domain_handle_id.is_some());
                let msg = m.attributes.as_ref().unwrap().message.as_ref().unwrap();
                assert_eq!(msg, &NvtxMessage::String("domain mark".into()));
            }
            other => panic!("expected Mark, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    // --- Domain-scoped push/pop ---
    {
        let _range = domain.local_range("domain local range");
    }

    {
        let captured = events.lock().unwrap();
        // nvidia-nvtx may emit a RegisterString before the Push.
        // Find Push and Pop events among whatever was emitted.
        let pushes: Vec<_> = captured
            .iter()
            .filter(|e| matches!(e, NvtxEvent::Push(_)))
            .collect();
        let pops: Vec<_> = captured
            .iter()
            .filter(|e| matches!(e, NvtxEvent::Pop(_)))
            .collect();
        assert_eq!(pushes.len(), 1, "expected 1 Push in domain push/pop");
        assert_eq!(pops.len(), 1, "expected 1 Pop in domain push/pop");
        match pushes[0] {
            NvtxEvent::Push(p) => {
                assert!(p.domain_handle_id.is_some());
            }
            _ => unreachable!(),
        }
        match pops[0] {
            NvtxEvent::Pop(p) => {
                assert!(p.domain_handle_id.is_some());
            }
            _ => unreachable!(),
        }
    }
    events.lock().unwrap().clear();

    // --- Registered string ---
    let reg = domain.register_string("registered msg");
    domain.mark(reg);

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 2, "expected RegisterString + Mark");
        match &captured[0] {
            NvtxEvent::RegisterString(r) => {
                assert!(r.domain_handle_id > 0);
                assert_eq!(r.value, "registered msg");
            }
            other => panic!("expected RegisterString, got {other:?}"),
        }
        match &captured[1] {
            NvtxEvent::Mark(m) => {
                let msg = m.attributes.as_ref().unwrap().message.as_ref().unwrap();
                match msg {
                    NvtxMessage::RegisteredHandle(id) => assert!(*id > 0),
                    other => panic!("expected RegisteredHandle, got {other:?}"),
                }
            }
            other => panic!("expected Mark, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    // --- Category ---
    let cat = domain.register_category("test-category");

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1, "expected 1 NameCategory event");
        match &captured[0] {
            NvtxEvent::NameCategory(nc) => {
                assert!(nc.domain_handle_id.is_some());
                assert_eq!(nc.name, "test-category");
                assert!(nc.category_id > 0);
            }
            other => panic!("expected NameCategory, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    // --- Mark with color and category via EventAttributes ---
    let attrs = domain
        .event_attributes_builder()
        .message("colored mark")
        .color(nvtx::Color::new(255, 0, 0, 255))
        .category(cat)
        .build();
    domain.mark(attrs);

    {
        let captured = events.lock().unwrap();
        // nvidia-nvtx may emit a RegisterString for the message before the Mark.
        let marks: Vec<_> = captured
            .iter()
            .filter(|e| matches!(e, NvtxEvent::Mark(_)))
            .collect();
        assert_eq!(marks.len(), 1, "expected 1 Mark in attributed mark");
        match marks[0] {
            NvtxEvent::Mark(m) => {
                let a = m.attributes.as_ref().unwrap();
                assert!(a.color.is_some());
                assert!(a.category_id > 0);
            }
            _ => unreachable!(),
        }
    }
    events.lock().unwrap().clear();

    // --- Domain destroy (drop) ---
    drop(domain);

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1, "expected 1 DomainDestroy event");
        match &captured[0] {
            NvtxEvent::DomainDestroy(d) => {
                assert!(d.domain_handle_id > 0);
            }
            other => panic!("expected DomainDestroy, got {other:?}"),
        }
    }
}
