// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed access to stored FSM transitions and states.

use quent_time::{Timestamp, span::SpanUnixNanoSec};

use crate::entity::EntityHandle;

pub mod native;

/// Provides an event sequence number.
pub trait Sequence {
    /// Returns the wrapping sequence number used to order events with equal timestamps.
    fn sequence(&self) -> u16;
}

/// Provides access to a recorded [`FsmHandle`] state transition.
pub trait Transition: Timestamp + Sequence {
    /// Return the unique name of the state this transition leads to.
    fn name(&self) -> &'static str;

    /// Returns whether this transition ends the FSM's dynamic lifetime.
    fn is_final(&self) -> bool;
}

/// Provides access to a stored Finite State Machine (FSM).
///
/// Every recorded entity event is a transition, ordered by timestamp and then
/// sequence number. Each adjacent pair delimits one state, which may have zero
/// duration when the timestamps are equal. A complete FSM has an entry
/// transition and a final transition that closes the last state and ends its
/// dynamic lifetime. Incomplete recordings may omit either boundary.
pub trait FsmHandle: EntityHandle {
    /// A handle borrowing a recorded transition.
    type Transition<'a>: Transition
    where
        Self: 'a;

    /// Returns the number of delimited states, one less than the event count.
    fn num_states(&self) -> usize {
        self.num_events().get() - 1
    }

    /// Returns whether this FSM has no delimited states.
    ///
    /// Returns `true` when only one transition event was recorded, since a
    /// state requires two consecutive transitions.
    fn is_empty(&self) -> bool {
        self.num_states() == 0
    }

    /// Returns a handle to the transition at the given index.
    ///
    /// Returns `Some` for every index below [`EntityHandle::num_events`] and
    /// `None` otherwise.
    fn transition(&self, index: usize) -> Option<Self::Transition<'_>>;

    /// Returns a handle to the state at the given index.
    ///
    /// The state spans from transition `index` to transition `index + 1`.
    /// Returns `None` if the index is out of bounds.
    fn state(&self, index: usize) -> Option<FsmStateHandle<Self::Transition<'_>>> {
        if index >= self.num_states() {
            return None;
        }
        Some(FsmStateHandle {
            opening: self.transition(index).unwrap(),
            closing: self.transition(index + 1).unwrap(),
        })
    }

    /// Returns an iterator over all delimited states in this FSM.
    fn states(&self) -> impl ExactSizeIterator<Item = FsmStateHandle<Self::Transition<'_>>> {
        (0..self.num_states()).map(|index| self.state(index).unwrap())
    }

    /// Returns the first state, if the FSM is not empty.
    fn first(&self) -> Option<FsmStateHandle<Self::Transition<'_>>> {
        self.state(0)
    }

    /// Returns the last state, if the FSM is not empty.
    fn last(&self) -> Option<FsmStateHandle<Self::Transition<'_>>> {
        self.num_states()
            .checked_sub(1)
            .and_then(|index| self.state(index))
    }
}

impl<T: FsmHandle + ?Sized> FsmHandle for &T {
    type Transition<'a>
        = T::Transition<'a>
    where
        Self: 'a;

    fn num_states(&self) -> usize {
        T::num_states(self)
    }

    fn is_empty(&self) -> bool {
        T::is_empty(self)
    }

    fn transition(&self, index: usize) -> Option<Self::Transition<'_>> {
        T::transition(self, index)
    }

    fn state(&self, index: usize) -> Option<FsmStateHandle<Self::Transition<'_>>> {
        T::state(self, index)
    }

    fn states(&self) -> impl ExactSizeIterator<Item = FsmStateHandle<Self::Transition<'_>>> {
        T::states(self)
    }

    fn first(&self) -> Option<FsmStateHandle<Self::Transition<'_>>> {
        T::first(self)
    }

    fn last(&self) -> Option<FsmStateHandle<Self::Transition<'_>>> {
        T::last(self)
    }
}

/// Borrows a delimited FSM state through its opening and closing transitions.
#[derive(Clone, Copy, Debug)]
pub struct FsmStateHandle<T: Transition> {
    opening: T,
    closing: T,
}

impl<T: Transition> FsmStateHandle<T> {
    /// Creates a state handle from its opening and closing transitions.
    ///
    /// Callers must supply consecutive transitions from the same FSM.
    /// Equal timestamps are allowed.
    ///
    /// # Errors
    ///
    /// Returns an error if the closing timestamp precedes the opening timestamp.
    pub fn try_new(opening: T, closing: T) -> quent_time::Result<Self> {
        SpanUnixNanoSec::try_new(opening.timestamp(), closing.timestamp())?;
        Ok(Self { opening, closing })
    }

    /// Returns the name of the state entered by the opening transition.
    pub fn name(&self) -> &'static str {
        self.opening.name()
    }

    /// Returns the transition that opens this state.
    pub fn opening_transition(&self) -> &T {
        &self.opening
    }

    /// Returns the transition that closes this state.
    ///
    /// For the last state of a complete FSM, this is the final transition.
    pub fn closing_transition(&self) -> &T {
        &self.closing
    }

    /// Returns the span between the opening and closing transitions.
    pub fn span(&self) -> SpanUnixNanoSec {
        SpanUnixNanoSec::try_new(self.opening.timestamp(), self.closing.timestamp()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, num::NonZeroUsize};

    use quent_events::{EntityMarker, EventPayload};
    use quent_time::TimeUnixNanoSec;
    use uuid::Uuid;

    use super::*;
    use crate::Id;

    struct TestEntity;

    impl EntityMarker for TestEntity {
        type Payload = TestTransition;
    }

    struct TestTransition(u16);

    impl EventPayload for TestTransition {
        const NAME: &'static str = "test";
    }

    impl Timestamp for TestTransition {
        fn timestamp(&self) -> TimeUnixNanoSec {
            self.0.into()
        }
    }

    impl Sequence for TestTransition {
        fn sequence(&self) -> u16 {
            self.0
        }
    }

    impl Transition for TestTransition {
        fn name(&self) -> &'static str {
            "test"
        }

        fn is_final(&self) -> bool {
            self.0 == 1
        }
    }

    #[derive(Default)]
    struct TestFsm {
        calls: Cell<[u8; 7]>,
    }

    impl TestFsm {
        fn record<T>(&self, method: usize, result: T) -> T {
            let mut calls = self.calls.get();
            calls[method] += 1;
            self.calls.set(calls);
            result
        }

        fn state_handle() -> FsmStateHandle<TestTransition> {
            FsmStateHandle::try_new(TestTransition(0), TestTransition(1)).unwrap()
        }
    }

    impl EntityHandle for TestFsm {
        type Entity = TestEntity;

        fn id(&self) -> Id<Self::Entity> {
            Id::new(Uuid::nil())
        }

        fn earliest_timestamp(&self) -> TimeUnixNanoSec {
            0
        }

        fn latest_timestamp(&self) -> TimeUnixNanoSec {
            1
        }

        fn num_events(&self) -> NonZeroUsize {
            NonZeroUsize::new(2).unwrap()
        }
    }

    impl FsmHandle for TestFsm {
        type Transition<'a> = TestTransition;

        fn num_states(&self) -> usize {
            self.record(0, 1)
        }

        fn is_empty(&self) -> bool {
            self.record(1, false)
        }

        fn transition(&self, index: usize) -> Option<Self::Transition<'_>> {
            self.record(2, (index < 2).then_some(TestTransition(index as u16)))
        }

        fn state(&self, index: usize) -> Option<FsmStateHandle<Self::Transition<'_>>> {
            self.record(3, (index == 0).then(Self::state_handle))
        }

        fn states(&self) -> impl ExactSizeIterator<Item = FsmStateHandle<Self::Transition<'_>>> {
            self.record(4, std::iter::once(Self::state_handle()))
        }

        fn first(&self) -> Option<FsmStateHandle<Self::Transition<'_>>> {
            self.record(5, Some(Self::state_handle()))
        }

        fn last(&self) -> Option<FsmStateHandle<Self::Transition<'_>>> {
            self.record(6, Some(Self::state_handle()))
        }
    }

    #[test]
    fn reference_preserves_all_fsm_overrides() {
        fn check(handle: impl FsmHandle) {
            assert_eq!(handle.num_states(), 1);
            assert!(!handle.is_empty());
            assert_eq!(handle.transition(0).unwrap().sequence(), 0);
            assert_eq!(handle.state(0).unwrap().span().duration(), 1);
            assert_eq!(handle.states().count(), 1);
            assert_eq!(handle.first().unwrap().opening_transition().sequence(), 0);
            assert!(handle.last().unwrap().closing_transition().is_final());
        }

        let fsm = TestFsm::default();
        check(&fsm);
        assert_eq!(fsm.calls.get(), [1; 7]);
    }
}
