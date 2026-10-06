// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runs instrumentation and loads its filesystem-exported events.

use demo::{Demo, NvtxEvent, Query};
use quent_store::event::filesystem::Store;
use quent_store::event::{EntityEventStore, ModelEventStore};

#[allow(unused_imports)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = tempfile::tempdir()?;
    let context_id = quent_instrumentation_build_example::run_with_ndjson(output.path())?;

    let store = Store::<Demo>::new(output.path());

    println!("--- Query events ---");

    // Load events for one entity type.
    for event in store.entity_events::<Query>(context_id)? {
        println!("{:?}", event?);
    }

    println!("\n--- NVTX events through the generated store ---");

    for event in store.entity_events::<NvtxEvent>(context_id)? {
        println!("{:?}", event?);
    }

    println!("\n--- All model events ---");

    // Load all model events as `DemoEvent`.
    for event in store.events(context_id)? {
        println!("{:?}", event?);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        num::NonZeroUsize,
        sync::Arc,
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use nvtx_analyzer::{NvtxMessageData, NvtxModelBuilder, NvtxPayloadValue};
    use quent_analyzer::context::ContextId;
    use quent_analyzer::service::{AnalysisCache, BlockingTasks};
    use quent_store::event::filesystem::StreamAvailability;

    use super::*;
    use crate::demo::{DemoEvent, NvtxEventEvent, ServerEvent};

    #[test]
    fn generated_model_shares_one_context_load_between_analysis_consumers() {
        let output = tempfile::tempdir().unwrap();
        let id = quent_instrumentation_build_example::run_with_ndjson(output.path()).unwrap();
        let store = Arc::new(Store::<Demo>::new(output.path()));
        let cache = AnalysisCache::new(
            8,
            Duration::from_secs(60),
            BlockingTasks::new(NonZeroUsize::new(2).unwrap()),
        );
        let loads = Arc::new(AtomicUsize::new(0));
        let load = || {
            let store = Arc::clone(&store);
            let loads = Arc::clone(&loads);
            move || {
                loads.fetch_add(1, Ordering::SeqCst);
                store.load_context(id)
            }
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (first, second) = runtime.block_on(async {
            tokio::join!(
                cache.get_with(ContextId::from(id), load()),
                cache.get_with(ContextId::from(id), load()),
            )
        });
        let (context, second) = (first.unwrap(), second.unwrap());
        assert!(Arc::ptr_eq(&context, &second));
        assert_eq!(loads.load(Ordering::SeqCst), 1);
        assert_eq!(context.id(), id);
        assert_eq!(
            context.stream_availability("Query"),
            StreamAvailability::Populated
        );
        assert_eq!(
            context.stream_availability("NvtxEvent"),
            StreamAvailability::Populated
        );
        assert!(!context.events().is_empty());

        let mut process_id = None;
        let mut bound_process_id = None;
        let mut stream_id = None;
        for event in context.events() {
            match &event.data {
                DemoEvent::Server(ServerEvent::Booted { .. }) => process_id = Some(event.id),
                DemoEvent::NvtxEvent(NvtxEventEvent::Initialized { process }) => {
                    bound_process_id = Some(process.target);
                    stream_id = Some(event.id);
                }
                DemoEvent::NvtxEvent(_) => {
                    assert_eq!(stream_id.get_or_insert(event.id), &event.id);
                }
                _ => {}
            }
        }
        assert_eq!(bound_process_id, process_id);

        let nvtx = NvtxModelBuilder::build_from(context.events().iter().filter_map(|event| {
            match &event.data {
                DemoEvent::NvtxEvent(data) => Some((event.timestamp, data)),
                _ => None,
            }
        }));
        assert_eq!(nvtx.spans().len(), 1);
        let span = &nvtx.spans()[0];
        assert_eq!(span.name, "generated-shared-io");
        assert_eq!(span.category, Some(u32::MAX));
        assert_eq!(span.color.unwrap().color_type, i32::MIN);
        assert_eq!(span.color.unwrap().value, u32::MAX);
        let payload = span.payload.unwrap();
        assert_eq!(payload.payload_type, i32::MAX);
        let NvtxPayloadValue::Double(value) = payload.value else {
            panic!("expected the generated double payload kind")
        };
        assert_eq!(value.to_bits(), 0x7ff8_0000_0000_1234);
        assert!(span.end.is_some());
        assert!(
            nvtx.threads()
                .iter()
                .any(|thread| thread.name == "demo-main")
        );

        let malformed_message = crate::demo::quent::nvtx::Message {
            kind: u8::MAX,
            string: None,
            registered_handle: None,
        };
        assert_eq!(malformed_message.nvtx_message(), None);
    }
}
