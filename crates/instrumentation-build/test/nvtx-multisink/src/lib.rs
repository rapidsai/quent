// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generated instrumentation fixture for a binding-aware NVTX backend.

#[allow(unused)]
mod model {
    include!(concat!(env!("OUT_DIR"), "/nvtxmultisinktest.rs"));
}

#[cfg(all(test, target_os = "linux", target_pointer_width = "64"))]
mod tests {
    use std::sync::{Arc, Mutex};

    use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};

    use crate::model::{
        Context, NvtxEventEvent, NvtxMultisinkTest, NvtxMultisinkTestEvent, Process, ProcessEvent,
        Uuid,
    };

    type Captured = Arc<Mutex<Vec<quent_instrumentation::Event<NvtxMultisinkTestEvent>>>>;

    fn callback(
        captured: &Captured,
    ) -> quent_instrumentation::EventCallback<NvtxMultisinkTestEvent> {
        let captured = Arc::clone(captured);
        quent_instrumentation::EventCallback::new(move |event| {
            captured
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(event);
        })
    }

    fn mark(label: &str) -> NvtxEvent {
        NvtxEvent::Mark {
            domain: 0,
            attributes: NvtxEventAttributes {
                message: Some(NvtxMessage::String(label.to_owned())),
                ..Default::default()
            },
        }
    }

    fn labels(events: &[quent_instrumentation::Event<NvtxMultisinkTestEvent>]) -> Vec<&str> {
        events
            .iter()
            .filter_map(|event| match &event.data {
                NvtxMultisinkTestEvent::NvtxEvent(NvtxEventEvent::Mark { attributes, .. }) => {
                    attributes
                        .message
                        .as_ref()
                        .and_then(|message| message.string.as_deref())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn binding_aware_backend_keeps_two_generated_contexts_isolated() {
        nvtx_injection::reset();

        let first_context_id = Uuid::from_u128(0x101);
        let second_context_id = Uuid::from_u128(0x202);
        let first_process_id = Uuid::from_u128(0x303);
        let second_process_id = Uuid::from_u128(0x404);
        let first_events = Arc::new(Mutex::new(Vec::new()));
        let second_events = Arc::new(Mutex::new(Vec::new()));

        let first =
            Context::<NvtxMultisinkTest>::try_with_id(first_context_id, callback(&first_events))
                .unwrap();
        let second =
            Context::<NvtxMultisinkTest>::try_with_id(second_context_id, callback(&second_events))
                .unwrap();
        assert_eq!(first.id(), first_context_id);
        assert_eq!(second.id(), second_context_id);

        let mut first_process = first.observer::<Process>().handle_with_id(first_process_id);
        let mut second_process = second
            .observer::<Process>()
            .handle_with_id(second_process_id);
        first_process
            .started(crate::model::quent::os::Process {
                native_id: std::process::id(),
            })
            .unwrap();
        second_process
            .started(crate::model::quent::os::Process {
                native_id: std::process::id(),
            })
            .unwrap();

        let bindings = nvtx_injection::bindings();
        assert_eq!(bindings.len(), 2);
        let first_binding = bindings
            .iter()
            .copied()
            .find(|binding| binding.context_id == first_context_id)
            .unwrap();
        let second_binding = bindings
            .iter()
            .copied()
            .find(|binding| binding.context_id == second_context_id)
            .unwrap();
        assert_eq!(first_binding.process_id, first_process_id);
        assert_eq!(second_binding.process_id, second_process_id);
        assert_ne!(first_binding.stream_id, second_binding.stream_id);

        assert!(nvtx_injection::dispatch_to(
            first_binding,
            mark("first-context")
        ));
        assert!(nvtx_injection::dispatch_to(
            second_binding,
            mark("second-context")
        ));

        drop(first_process);
        drop(second_process);
        drop(first);
        drop(second);

        let first_events = first_events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let second_events = second_events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        assert!(first_events.iter().any(|event| {
            event.id == first_process_id
                && matches!(
                    &event.data,
                    NvtxMultisinkTestEvent::Process(ProcessEvent::Started { .. })
                )
        }));
        assert!(second_events.iter().any(|event| {
            event.id == second_process_id
                && matches!(
                    &event.data,
                    NvtxMultisinkTestEvent::Process(ProcessEvent::Started { .. })
                )
        }));
        assert!(first_events.iter().any(|event| {
            event.id == first_binding.stream_id
                && matches!(
                    &event.data,
                    NvtxMultisinkTestEvent::NvtxEvent(NvtxEventEvent::Initialized { process })
                        if process.target == first_process_id
                )
        }));
        assert!(second_events.iter().any(|event| {
            event.id == second_binding.stream_id
                && matches!(
                    &event.data,
                    NvtxMultisinkTestEvent::NvtxEvent(NvtxEventEvent::Initialized { process })
                        if process.target == second_process_id
                )
        }));
        assert_eq!(labels(&first_events), ["first-context"]);
        assert_eq!(labels(&second_events), ["second-context"]);
        assert!(first_events.iter().all(|event| {
            !matches!(&event.data, NvtxMultisinkTestEvent::NvtxEvent(_))
                || event.id == first_binding.stream_id
        }));
        assert!(second_events.iter().all(|event| {
            !matches!(&event.data, NvtxMultisinkTestEvent::NvtxEvent(_))
                || event.id == second_binding.stream_id
        }));

        drop(first_events);
        drop(second_events);
        nvtx_injection::reset();
    }
}
