// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runs instrumentation and loads its filesystem-exported events.

use demo::{Demo, Query};
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

    use quent_analyzer::context::ContextId;
    use quent_analyzer::service::{AnalysisCache, BlockingTasks};
    use quent_store::event::filesystem::StreamAvailability;

    use super::*;

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
            StreamAvailability::Undeclared
        );
        assert!(!context.events().is_empty());
    }
}
