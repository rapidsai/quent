// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! An exporter that hands each event to a caller-supplied callback. Intended
//! for tests that collect emitted events in memory.

use std::any::Any;
use std::sync::Arc;

use quent_events::{EntityEvent, Event};
use quent_exporter_types::{Exporter, ExporterResult};
use serde::Serialize;

/// One exported event, type-erased so a single callback can receive events of
/// any entity type. `event` is a boxed `Event<T>` for the entity named by
/// `entity`; downcast it with `event.downcast::<Event<T>>()`.
pub struct RecordedEvent {
    pub entity: &'static str,
    pub event: Box<dyn Any + Send>,
}

/// A thread-safe callback invoked once per exported event.
#[derive(Clone)]
pub struct EventCallback(Arc<dyn Fn(RecordedEvent) + Send + Sync>);

impl EventCallback {
    pub fn new(callback: impl Fn(RecordedEvent) + Send + Sync + 'static) -> Self {
        Self(Arc::new(callback))
    }
}

impl std::fmt::Debug for EventCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventCallback").finish_non_exhaustive()
    }
}

/// Type-erases each pushed event and forwards it to an [`EventCallback`].
pub struct CallbackExporter {
    callback: EventCallback,
}

impl CallbackExporter {
    pub fn new(callback: EventCallback) -> Self {
        Self { callback }
    }
}

#[async_trait::async_trait]
impl<T> Exporter<T> for CallbackExporter
where
    T: Serialize + Send + EntityEvent + 'static,
{
    async fn push(&mut self, event: Event<T>) -> ExporterResult<()> {
        (self.callback.0)(RecordedEvent {
            entity: T::NAME,
            event: Box::new(event),
        });
        Ok(())
    }

    async fn shutdown(&mut self) -> ExporterResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use uuid::Uuid;

    use super::*;

    #[derive(Serialize)]
    struct Alpha {
        a: u32,
    }
    impl EntityEvent for Alpha {
        const NAME: &'static str = "alpha";
    }

    #[derive(Serialize)]
    struct Beta {
        b: String,
    }
    impl EntityEvent for Beta {
        const NAME: &'static str = "beta";
    }

    // One shared callback receives events from exporters of two distinct entity
    // types; each erased event must downcast back to its concrete `Event<T>`.
    #[tokio::test]
    async fn forwards_multiple_event_types_erased() {
        let recorded = Arc::new(Mutex::new(Vec::<RecordedEvent>::new()));
        let callback = {
            let recorded = recorded.clone();
            EventCallback::new(move |rec| recorded.lock().unwrap().push(rec))
        };

        let mut alpha: Box<dyn Exporter<Alpha>> = Box::new(CallbackExporter::new(callback.clone()));
        let mut beta: Box<dyn Exporter<Beta>> = Box::new(CallbackExporter::new(callback.clone()));

        let (id0, id1, id2) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
        alpha
            .push(Event::new(id0, 10, Alpha { a: 7 }))
            .await
            .unwrap();
        beta.push(Event::new(id1, 20, Beta { b: "x".into() }))
            .await
            .unwrap();
        alpha
            .push(Event::new(id2, 30, Alpha { a: 9 }))
            .await
            .unwrap();

        let recorded = recorded.lock().unwrap();
        assert_eq!(recorded.len(), 3);

        // Entity names are preserved in emission order.
        assert_eq!(
            recorded.iter().map(|r| r.entity).collect::<Vec<_>>(),
            [Alpha::NAME, Beta::NAME, Alpha::NAME],
        );

        let a0 = recorded[0].event.downcast_ref::<Event<Alpha>>().unwrap();
        assert_eq!((a0.id, a0.timestamp, a0.data.a), (id0, 10, 7));

        let b1 = recorded[1].event.downcast_ref::<Event<Beta>>().unwrap();
        assert_eq!((b1.id, b1.timestamp, b1.data.b.as_str()), (id1, 20, "x"));

        let a2 = recorded[2].event.downcast_ref::<Event<Alpha>>().unwrap();
        assert_eq!((a2.id, a2.data.a), (id2, 9));

        // A mismatched concrete type does not downcast.
        assert!(recorded[1].event.downcast_ref::<Event<Alpha>>().is_none());
    }
}
