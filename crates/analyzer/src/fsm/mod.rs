// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! FSM analysis interfaces and storage implementations.

use quent_time::{Timestamp, span::SpanUnixNanoSec};

use crate::{Entity, resource::Usage};

pub mod collection;
pub mod native;
pub mod runtime;

/// Trait for types that represent an [`Fsm`] state transition event payload.
pub trait Transition: Timestamp {
    /// Return the unique name of the state this transition leads to.
    fn name(&self) -> &str;

    /// Returns the per-FSM wrapping sequence number assigned in transition order.
    fn sequence(&self) -> u16;

    /// Returns whether this transition ends the FSM's dynamic lifetime.
    fn is_final(&self) -> bool;
}

/// Trait for types that represent a Finite State Machine (FSM).
///
/// An FSM is modeled as an ordered sequence of transitions, where each adjacent
/// pair delimits one state. A complete FSM has an entry transition and a final
/// transition that closes the last state and ends its dynamic lifetime.
///
/// This trait may also represent an incomplete FSM with no delimited states.
pub trait Fsm: Entity {
    /// The type of the transition event payload of this FSM.
    ///
    /// This associated type enables dyn-free access to underlying transition
    /// data.
    type TransitionType: Transition;

    /// Return the number of states in this FSM.
    ///
    /// Each state spans two consecutive transitions, so this is always one less
    /// than the number of transition events.
    fn len(&self) -> usize;

    /// Return true if this FSM has no states (meaning the model of whatever it
    /// represents is incomplete).
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return a reference to the transition at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    fn transition(&self, index: usize) -> Option<&Self::TransitionType>;

    /// Return a reference to the state at the given index.
    ///
    /// The state spans from transition `index` to transition `index + 1`.
    /// Returns `None` if the index is out of bounds.
    fn state<'a>(&'a self, index: usize) -> Option<FsmStateRef<'a, Self, Self::TransitionType>> {
        (self.len() > index).then_some(FsmStateRef { fsm: self, index })
    }

    /// Return an iterator over all states in this FSM.
    fn states<'a>(
        &'a self,
    ) -> impl ExactSizeIterator<Item = FsmStateRef<'a, Self, Self::TransitionType>> {
        (0..self.len()).map(|index| self.state(index).unwrap())
    }

    /// Return the first state, if the FSM is not empty.
    fn first<'a>(&'a self) -> Option<FsmStateRef<'a, Self, Self::TransitionType>> {
        self.state(0)
    }

    /// Return the last state, if the FSM is not empty.
    fn last<'a>(&'a self) -> Option<FsmStateRef<'a, Self, Self::TransitionType>> {
        self.len()
            .checked_sub(1)
            .and_then(|index| self.state(index))
    }
}

#[cfg(test)]
mod tests {
    use quent_time::TimeUnixNanoSec;
    use uuid::Uuid;

    use super::*;

    struct TestTransition {
        timestamp: TimeUnixNanoSec,
        final_transition: bool,
    }

    impl Timestamp for TestTransition {
        fn timestamp(&self) -> TimeUnixNanoSec {
            self.timestamp
        }
    }

    impl Transition for TestTransition {
        fn name(&self) -> &str {
            "test"
        }

        fn sequence(&self) -> u16 {
            0
        }

        fn is_final(&self) -> bool {
            self.final_transition
        }
    }

    struct TestFsm(Vec<TestTransition>);

    impl Entity for TestFsm {
        fn id(&self) -> Uuid {
            Uuid::nil()
        }

        fn type_name(&self) -> &str {
            "test"
        }

        fn earliest_timestamp(&self) -> TimeUnixNanoSec {
            0
        }

        fn latest_timestamp(&self) -> TimeUnixNanoSec {
            0
        }
    }

    impl Fsm for TestFsm {
        type TransitionType = TestTransition;

        fn len(&self) -> usize {
            self.0.len().saturating_sub(1)
        }

        fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
            self.0.get(index)
        }
    }

    #[test]
    fn empty_fsm_has_no_last_state() {
        assert!(TestFsm(Vec::new()).last().is_none());
    }

    #[test]
    fn last_state_is_closed_by_final_transition() {
        let fsm = TestFsm(vec![
            TestTransition {
                timestamp: 1,
                final_transition: false,
            },
            TestTransition {
                timestamp: 2,
                final_transition: true,
            },
        ]);

        let last_state = fsm.last().unwrap();
        let closing_transition = last_state.next_transition();
        assert_eq!(closing_transition.timestamp(), 2);
        assert!(closing_transition.is_final());
    }
}

/// Trait for FSMs that have resource usages associated with their states.
pub trait FsmUsages<'a>: Fsm {
    /// Return an iterator over all usages with their associated state names.
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)>;
}

#[derive(Clone)]
pub struct FsmStateRef<'a, F, T>
where
    F: Fsm<TransitionType = T> + ?Sized,
    T: Transition,
{
    fsm: &'a F,
    index: usize,
}

impl<'a, F, T> FsmStateRef<'a, F, T>
where
    F: Fsm<TransitionType = T>,
    T: Transition,
{
    pub fn name(&self) -> &str {
        self.fsm.transition(self.index).unwrap().name()
    }

    /// Returns the transition that closes this state.
    ///
    /// For the last state of a complete FSM, this is the final transition.
    pub fn next_transition(&self) -> &T {
        self.fsm.transition(self.index + 1).unwrap()
    }

    pub fn span(&self) -> SpanUnixNanoSec {
        let start = self.fsm.transition(self.index).unwrap().timestamp();
        let end = self.next_transition().timestamp();
        SpanUnixNanoSec::try_new(start, end).unwrap()
    }
}
