// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Backing types for generated FSM instrumentation handles.

use crate::{AnyEntity, EntityRef, HandleInner, InstrumentedEntity, Uuid};

/// Assigns the transition sequence carried by an FSM event.
#[doc(hidden)]
pub trait FsmEvent {
    /// Replaces the event's transition sequence.
    fn set_sequence(&mut self, sequence: u16);
}

/// Maps a generated typestate marker to its dynamic FSM state.
#[doc(hidden)]
pub trait FsmState<Entity> {
    /// Identifies this state in a dynamic handle.
    ///
    /// States are numbered from 1 to 255. Zero means that the FSM has not
    /// entered its initial state yet. Code generation rejects an FSM entity
    /// that declares 256 or more states, so every state index fits in a `u8`.
    #[doc(hidden)]
    const DYNAMIC_STATE_INDEX: u8;

    /// Schema name of this state.
    #[doc(hidden)]
    const NAME: &'static str;
}

/// An invalid transition attempted through a dynamic-state FSM handle.
#[derive(Debug, thiserror::Error)]
#[error("cannot transition `{entity}` from `{state}` to `{target}`")]
pub struct FsmTransitionError {
    entity: &'static str,
    state: &'static str,
    target: &'static str,
}

/// A dynamic FSM handle whose state did not match a requested typestate.
pub struct FsmStateMismatch<H> {
    handle: H,
    entity: &'static str,
    state: &'static str,
    expected: &'static str,
}

impl<H> FsmStateMismatch<H> {
    /// Creates an error that retains the dynamic handle.
    #[doc(hidden)]
    pub fn new(
        handle: H,
        entity: &'static str,
        state: &'static str,
        expected: &'static str,
    ) -> Self {
        Self {
            handle,
            entity,
            state,
            expected,
        }
    }

    /// Borrows the dynamic handle.
    pub fn handle(&self) -> &H {
        &self.handle
    }

    /// Consumes this error and returns the dynamic handle.
    pub fn into_handle(self) -> H {
        self.handle
    }
}

impl<H> ::core::fmt::Debug for FsmStateMismatch<H> {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        formatter
            .debug_struct("FsmStateMismatch")
            .field("entity", &self.entity)
            .field("state", &self.state)
            .field("expected", &self.expected)
            .finish_non_exhaustive()
    }
}

impl<H> ::core::fmt::Display for FsmStateMismatch<H> {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(
            formatter,
            "cannot convert `{}` FSM from dynamic state `{}` to typestate `{}`",
            self.entity, self.state, self.expected,
        )
    }
}

impl<H> ::std::error::Error for FsmStateMismatch<H> {}

impl FsmTransitionError {
    /// Creates an error for an invalid dynamic-state transition.
    #[doc(hidden)]
    pub fn new(entity: &'static str, state: &'static str, target: &'static str) -> Self {
        Self {
            entity,
            state,
            target,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SequenceCounter(u16);

impl SequenceCounter {
    fn advance(self) -> (u16, Self) {
        (self.0, Self(self.0.wrapping_add(1)))
    }
}

/// Owns the runtime state of a generated FSM handle.
#[doc(hidden)]
pub struct FsmHandleInner<E: InstrumentedEntity> {
    handle: HandleInner<E>,
    sequence: SequenceCounter,
}

impl<E: InstrumentedEntity> From<HandleInner<E>> for FsmHandleInner<E> {
    fn from(handle: HandleInner<E>) -> Self {
        Self {
            handle,
            sequence: SequenceCounter::default(),
        }
    }
}

impl<E> FsmHandleInner<E>
where
    E: InstrumentedEntity,
    E::Payload: FsmEvent,
{
    /// Emits `event` with the next wrapping transition sequence number.
    ///
    /// Hidden because generated instrumentation uses this primitive to implement
    /// in-place transitions on dynamic-state FSM handles; application code uses
    /// the generated transition methods instead.
    #[doc(hidden)]
    pub fn transition_mut(&mut self, mut event: E::Payload) {
        let (sequence, next) = self.sequence.advance();
        event.set_sequence(sequence);
        self.handle.emit(event);
        self.sequence = next;
    }

    /// Emits `event` with the next wrapping transition sequence number.
    ///
    /// Hidden because generated instrumentation uses this consuming primitive
    /// to implement typestate transitions that return the target-state handle;
    /// application code uses the generated transition methods instead.
    #[doc(hidden)]
    pub fn transition(mut self, event: E::Payload) -> Self {
        self.transition_mut(event);
        self
    }
}

impl<E: InstrumentedEntity> FsmHandleInner<E> {
    /// Returns whether generated dynamic transitions must validate their source state.
    ///
    /// No-op observers disable transition checks; active observers retain them.
    /// Hidden because generated models use this predicate across crate boundaries.
    #[doc(hidden)]
    pub fn checks_transitions(&self) -> bool {
        !self.handle.is_noop()
    }

    /// Returns the entity instance ID.
    pub fn id(&self) -> Uuid {
        self.handle.id()
    }

    /// Returns a typed reference to this instance carrying no data.
    pub fn as_entity_ref(&self) -> EntityRef<E> {
        self.handle.as_entity_ref()
    }

    /// Returns a typed reference to this instance carrying `data`.
    pub fn as_entity_ref_with<T>(&self, data: T) -> EntityRef<E, T> {
        self.handle.as_entity_ref_with(data)
    }

    /// Returns an untyped reference to this instance carrying no data.
    pub fn as_any_entity_ref(&self) -> EntityRef<AnyEntity> {
        self.handle.as_any_entity_ref()
    }

    /// Returns an untyped reference to this instance carrying `data`.
    pub fn as_any_entity_ref_with<T>(&self, data: T) -> EntityRef<AnyEntity, T> {
        self.handle.as_any_entity_ref_with(data)
    }
}

#[cfg(test)]
mod tests {
    use super::SequenceCounter;

    #[test]
    fn sequence_counter_starts_at_zero_and_advances() {
        let (sequence, counter) = SequenceCounter::default().advance();
        assert_eq!(sequence, 0);
        let (sequence, _) = counter.advance();
        assert_eq!(sequence, 1);
    }

    #[test]
    fn sequence_counter_wraps() {
        let (sequence, counter) = SequenceCounter(u16::MAX).advance();
        assert_eq!(sequence, u16::MAX);
        let (sequence, _) = counter.advance();
        assert_eq!(sequence, 0);
    }
}
