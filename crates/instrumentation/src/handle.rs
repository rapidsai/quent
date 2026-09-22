// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Backing handle types used by generated instrumentation libraries.

use std::sync::Arc;

use crate::{InstrumentedEntity, ObserverInner};

/// An error from emitting through a generated entity handle.
#[derive(Debug, thiserror::Error)]
pub enum HandleError {
    /// A once-cardinality event was emitted more than once for one entity
    /// instance.
    #[error("once-event `{event}` already emitted for this entity instance")]
    OnceAlreadyEmitted {
        /// Name of the event that was re-emitted.
        event: &'static str,
    },
    /// A private event source could not be prepared or activated.
    #[error("source activation failed: {source}")]
    SourceActivation {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl HandleError {
    /// Wrap an error returned while activating a generated private source.
    #[doc(hidden)]
    pub fn source_activation(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::SourceActivation {
            source: Box::new(source),
        }
    }
}

/// Common operations for generated handles.
///
/// Generated local newtypes wrap this type so they can add inherent
/// entity-specific event methods.
///
/// Hidden because generated handle newtypes are the application-facing API.
#[doc(hidden)]
pub struct HandleInner<E: InstrumentedEntity> {
    id: crate::Uuid,
    /// One bit per once-cardinality event, set once that event is emitted.
    once_flags: u64,
    observer: Arc<ObserverInner<E::Event>>,
}

impl<E: InstrumentedEntity> HandleInner<E> {
    pub(crate) fn new(observer: Arc<ObserverInner<E::Event>>) -> Self {
        Self::with_id(crate::Uuid::now_v7(), observer)
    }

    pub(crate) fn with_id(id: crate::Uuid, observer: Arc<ObserverInner<E::Event>>) -> Self {
        Self {
            id,
            once_flags: 0,
            observer,
        }
    }

    /// Returns the entity instance ID.
    pub fn uuid(&self) -> crate::Uuid {
        self.id
    }

    /// Returns a typed reference to this instance carrying no data.
    pub fn as_entity_ref(&self) -> crate::EntityRef<E> {
        crate::EntityRef::new(self.uuid(), ())
    }

    /// Returns a typed reference to this instance carrying `data`.
    pub fn as_entity_ref_with<T>(&self, data: T) -> crate::EntityRef<E, T> {
        crate::EntityRef::new(self.uuid(), data)
    }

    /// Returns an untyped reference to this instance carrying no data.
    pub fn as_any_entity_ref(&self) -> crate::EntityRef<crate::AnyEntity> {
        crate::EntityRef::new(self.uuid(), ())
    }

    /// Returns an untyped reference to this instance carrying `data`.
    pub fn as_any_entity_ref_with<T>(&self, data: T) -> crate::EntityRef<crate::AnyEntity, T> {
        crate::EntityRef::new(self.uuid(), data)
    }

    /// Emits an event without cardinality tracking.
    ///
    /// Hidden because generated event methods provide the typed API.
    #[doc(hidden)]
    pub fn emit(&self, event: E::Event) {
        self.observer.emit(self.id, event);
    }

    /// Emits an event unless the bit at `INDEX` was previously set.
    ///
    /// Hidden because generated once-event methods provide the typed API.
    ///
    /// # Errors
    ///
    /// Returns [`HandleError`](crate::HandleError) when the event was already emitted.
    #[doc(hidden)]
    pub fn emit_once<const INDEX: u32>(
        &mut self,
        event_name: &'static str,
        event: E::Event,
    ) -> Result<(), HandleError> {
        const { assert!(INDEX < u64::BITS, "once-event bit index out of range") };
        let mask = 1u64 << INDEX;
        if self.once_flags & mask != 0 {
            return Err(HandleError::OnceAlreadyEmitted { event: event_name });
        }
        self.once_flags |= mask;
        self.observer.emit(self.id, event);
        Ok(())
    }

    /// Emits a once-cardinality event, then invokes the observer's configured
    /// source activation callback.
    ///
    /// The native process identifier is validated before emission. The event is
    /// then queued before activation. An activation error is returned to the
    /// generated event method without rolling back the emitted identity event
    /// or its once flag.
    #[doc(hidden)]
    pub fn emit_once_and_activate<const INDEX: u32>(
        &mut self,
        event_name: &'static str,
        native_process_id: u32,
        event: E::Event,
    ) -> Result<(), HandleError> {
        self.observer
            .validate_native_process_id(native_process_id)?;
        self.emit_once::<INDEX>(event_name, event)?;
        self.observer.activate_after_emit(self.id)
    }

    /// Returns whether the bit at `INDEX` has been set.
    ///
    /// Hidden because generated once-event methods expose named checks.
    #[doc(hidden)]
    pub fn is_emitted<const INDEX: u32>(&self) -> bool {
        const { assert!(INDEX < u64::BITS, "once-event bit index out of range") };
        self.once_flags & (1u64 << INDEX) != 0
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use quent_events::{Entity, EntityEvent, Event};

    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestEvent(u8);

    impl EntityEvent for TestEvent {
        const NAME: &'static str = "Test";
    }

    struct TestEntity;

    impl Entity for TestEntity {
        type Event = TestEvent;
    }

    struct TestHandle;

    impl From<HandleInner<TestEntity>> for TestHandle {
        fn from(_: HandleInner<TestEntity>) -> Self {
            Self
        }
    }

    impl InstrumentedEntity for TestEntity {
        type Context = ();
        type Handle = TestHandle;
    }

    #[test]
    fn activation_runs_after_the_once_event_is_queued() {
        let (observer, receiver) = ObserverInner::active_for_test(true);
        let receiver = Arc::new(Mutex::new(receiver));
        let id = crate::Uuid::now_v7();
        let observed_id = Arc::new(Mutex::new(None));
        let activation = {
            let receiver = Arc::clone(&receiver);
            let observed_id = Arc::clone(&observed_id);
            move |activated_id| {
                let queued = receiver.lock().unwrap().try_recv().unwrap();
                assert_eq!(queued.id, activated_id);
                assert_eq!(queued.data, TestEvent(7));
                *observed_id.lock().unwrap() = Some(activated_id);
                Ok(())
            }
        };
        let observer = Arc::new(observer.with_emit_activation(activation));
        let mut handle = HandleInner::<TestEntity>::with_id(id, observer);

        handle
            .emit_once_and_activate::<0>("started", std::process::id(), TestEvent(7))
            .unwrap();

        assert_eq!(*observed_id.lock().unwrap(), Some(id));
    }

    #[test]
    fn activation_error_is_returned_after_the_event_is_queued() {
        let (observer, mut receiver) = ObserverInner::active_for_test(true);
        let observer = Arc::new(observer.with_emit_activation(|_| {
            Err(HandleError::source_activation(std::io::Error::other(
                "private source already installed",
            )))
        }));
        let mut handle = HandleInner::<TestEntity>::new(observer);

        let error = handle
            .emit_once_and_activate::<0>("started", std::process::id(), TestEvent(9))
            .unwrap_err();

        assert!(matches!(error, HandleError::SourceActivation { .. }));
        assert_eq!(receiver.try_recv().unwrap().data, TestEvent(9));
        assert!(handle.is_emitted::<0>());
    }

    #[test]
    fn noop_observer_discards_activation() {
        let activations = Arc::new(AtomicUsize::new(0));
        let observer = ObserverInner::noop().with_emit_activation({
            let activations = Arc::clone(&activations);
            move |_| {
                activations.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });
        let mut handle = HandleInner::<TestEntity>::new(Arc::new(observer));

        handle
            .emit_once_and_activate::<0>("started", u32::MAX, TestEvent(1))
            .unwrap();

        assert_eq!(activations.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn forwarding_does_not_activate_the_source() {
        let (observer, mut receiver) = ObserverInner::active_for_test(true);
        let activations = Arc::new(AtomicUsize::new(0));
        let observer = observer.with_emit_activation({
            let activations = Arc::clone(&activations);
            move |_| {
                activations.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });
        let id = crate::Uuid::now_v7();

        observer.send(Event::new(id, 42, TestEvent(3)));

        assert_eq!(receiver.try_recv().unwrap().data, TestEvent(3));
        assert_eq!(activations.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn disabled_source_capture_discards_activation() {
        let (observer, mut receiver) = ObserverInner::active_for_test(false);
        let activations = Arc::new(AtomicUsize::new(0));
        let observer = observer.with_emit_activation({
            let activations = Arc::clone(&activations);
            move |_| {
                activations.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });
        let mut handle = HandleInner::<TestEntity>::new(Arc::new(observer));

        handle
            .emit_once_and_activate::<0>("started", u32::MAX, TestEvent(4))
            .unwrap();

        assert_eq!(receiver.try_recv().unwrap().data, TestEvent(4));
        assert_eq!(activations.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn foreign_process_id_fails_before_emission_and_can_be_retried() {
        let (observer, mut receiver) = ObserverInner::active_for_test(true);
        let activations = Arc::new(AtomicUsize::new(0));
        let observer = observer.with_emit_activation({
            let activations = Arc::clone(&activations);
            move |_| {
                activations.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });
        let mut handle = HandleInner::<TestEntity>::new(Arc::new(observer));
        let foreign = std::process::id().wrapping_add(1);

        let error = handle
            .emit_once_and_activate::<0>("started", foreign, TestEvent(5))
            .unwrap_err();

        assert!(matches!(error, HandleError::SourceActivation { .. }));
        assert!(!handle.is_emitted::<0>());
        assert!(receiver.try_recv().is_err());
        assert_eq!(activations.load(Ordering::SeqCst), 0);

        handle
            .emit_once_and_activate::<0>("started", std::process::id(), TestEvent(6))
            .unwrap();
        assert!(handle.is_emitted::<0>());
        assert_eq!(receiver.try_recv().unwrap().data, TestEvent(6));
        assert_eq!(activations.load(Ordering::SeqCst), 1);
    }
}
