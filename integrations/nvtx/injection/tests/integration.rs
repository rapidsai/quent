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

    // --- Mark ---
    nvtx::mark("test mark");

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1, "expected 1 event after mark");
        match &captured[0] {
            NvtxEvent::Mark(m) => {
                assert!(m.thread_id > 0);
                assert!(m.domain_handle_id.is_none());
                let msg = m.attributes.as_ref().unwrap().message.as_ref().unwrap();
                match msg {
                    NvtxMessage::Ascii(s) => assert_eq!(s, "test mark"),
                    _ => panic!("expected Ascii message"),
                }
            }
            other => panic!("expected Mark, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    // --- Push/Pop (LocalRange) ---
    {
        let _range = nvtx::LocalRange::new("my range");
    }

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 2, "expected 2 events after push/pop");
        match &captured[0] {
            NvtxEvent::Push(p) => {
                assert!(p.thread_id > 0);
                let msg = p.attributes.as_ref().unwrap().message.as_ref().unwrap();
                match msg {
                    NvtxMessage::Ascii(s) => assert_eq!(s, "my range"),
                    _ => panic!("expected Ascii message on Push"),
                }
            }
            other => panic!("expected Push, got {other:?}"),
        }
        match &captured[1] {
            NvtxEvent::Pop(p) => {
                assert!(p.thread_id > 0);
            }
            other => panic!("expected Pop, got {other:?}"),
        }
    }
    events.lock().unwrap().clear();

    // --- Start/End (Range) ---
    {
        let _range = nvtx::Range::new("process range");
    }

    {
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 2, "expected 2 events after start/end");
        let start_id = match &captured[0] {
            NvtxEvent::RangeStart(r) => {
                let msg = r.attributes.as_ref().unwrap().message.as_ref().unwrap();
                match msg {
                    NvtxMessage::Ascii(s) => assert_eq!(s, "process range"),
                    _ => panic!("expected Ascii message on RangeStart"),
                }
                r.range_handle_id
            }
            other => panic!("expected RangeStart, got {other:?}"),
        };
        match &captured[1] {
            NvtxEvent::RangeEnd(r) => {
                assert_eq!(r.range_handle_id, start_id);
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
}
