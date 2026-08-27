// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! An exporter that hands each event to a caller-supplied callback. Intended
//! for tests that collect emitted events in memory.

use std::sync::Arc;

use quent_events::Event;
use quent_io_types::{Exporter, ExporterProvider, ExporterResult};

/// A thread-safe callback invoked once per exported event.
pub struct EventCallback<T>(Arc<dyn Fn(Event<T>) + Send + Sync>);

impl<T> Clone for EventCallback<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> EventCallback<T> {
    pub fn new(callback: impl Fn(Event<T>) + Send + Sync + 'static) -> Self {
        Self(Arc::new(callback))
    }
}

impl<T> std::fmt::Debug for EventCallback<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventCallback").finish_non_exhaustive()
    }
}

/// Converts each pushed event and forwards it to an [`EventCallback`].
pub struct CallbackExporter<T> {
    callback: EventCallback<T>,
}

impl<T> CallbackExporter<T> {
    pub fn new(callback: EventCallback<T>) -> Self {
        Self { callback }
    }
}

#[async_trait::async_trait]
impl<S, T> Exporter<S> for CallbackExporter<T>
where
    S: Into<T> + Send + 'static,
    T: Send + 'static,
{
    async fn push(&mut self, event: Event<S>) -> ExporterResult<()> {
        (self.callback.0)(Event::new(event.id, event.timestamp, event.data.into()));
        Ok(())
    }

    async fn shutdown(self: Box<Self>) -> ExporterResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl<S, T> ExporterProvider<S> for EventCallback<T>
where
    S: Into<T> + Send + 'static,
    T: Send + 'static,
{
    async fn create_exporter(
        &self,
        _context_id: uuid::Uuid,
    ) -> ExporterResult<Box<dyn Exporter<S>>> {
        Ok(Box::new(CallbackExporter::<T>::new(self.clone())))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use uuid::Uuid;

    use super::*;

    enum ModelEvent {
        Alpha(Alpha),
        Beta(Beta),
    }

    struct Alpha {
        a: u32,
    }
    impl From<Alpha> for ModelEvent {
        fn from(value: Alpha) -> Self {
            Self::Alpha(value)
        }
    }

    struct Beta {
        b: String,
    }
    impl From<Beta> for ModelEvent {
        fn from(value: Beta) -> Self {
            Self::Beta(value)
        }
    }

    #[tokio::test]
    async fn forwards_multiple_event_types_as_model_events() {
        let recorded = Arc::new(Mutex::new(Vec::<Event<ModelEvent>>::new()));
        let callback = {
            let recorded = recorded.clone();
            EventCallback::new(move |rec| recorded.lock().unwrap().push(rec))
        };

        let mut alpha: Box<dyn Exporter<Alpha>> =
            callback.create_exporter(Uuid::from_u128(10)).await.unwrap();
        let mut beta: Box<dyn Exporter<Beta>> =
            callback.create_exporter(Uuid::from_u128(10)).await.unwrap();

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

        assert_eq!((recorded[0].id, recorded[0].timestamp), (id0, 10));
        assert!(matches!(
            recorded[0].data,
            ModelEvent::Alpha(Alpha { a: 7 })
        ));
        assert_eq!((recorded[1].id, recorded[1].timestamp), (id1, 20));
        assert!(matches!(&recorded[1].data, ModelEvent::Beta(Beta { b }) if b == "x"));
        assert_eq!(recorded[2].id, id2);
        assert!(matches!(
            recorded[2].data,
            ModelEvent::Alpha(Alpha { a: 9 })
        ));
    }
}
