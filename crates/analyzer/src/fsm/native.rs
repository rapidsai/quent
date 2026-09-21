// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Analysis-time FSMs in Rust-native in-memory storage.

pub use quent_dynamic_attributes::DynamicAttribute;
use quent_events::{EntityEvent, Event};
use quent_time::{OrderKey, OrderedCollector, TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, FsmUsages, Transition},
    resource::{AnalyzedUsage, CapacityValue, Usage, Using},
};

/// Number of transitions stored inline before spilling to the heap.
///
/// The capacity is based on intuition rather than measurements.
const INLINE_TRANSITION_CAPACITY: usize = 4;

/// Trait for application-specific payloads of FSM transition events.
pub trait TransitionEvent: EntityEvent {
    /// Return the name of the state transitioned into.
    fn name(&self) -> &'static str;

    /// Returns the per-FSM wrapping sequence number assigned in transition order.
    fn sequence(&self) -> u16;

    /// Returns whether this transition starts the FSM's dynamic lifetime.
    fn is_initial(&self) -> bool;

    /// Returns whether this transition ends the FSM's dynamic lifetime.
    fn is_final(&self) -> bool;

    /// Returns whether `next` may immediately follow this transition.
    fn is_valid_next(&self, next: &Self) -> bool;

    /// Returns resources held until the next transition.
    fn usages(&self) -> SmallVec<[AnalyzedUsage; 1]> {
        SmallVec::new()
    }
}

/// Rust-native struct wrapping around an application-specific FSM transition
/// event payload.
///
/// This structure exists to pre-compute and cache generic properties of
/// transitions that are typically repeatedly requested by analysis consumers.
pub struct AnalyzedTransition<T> {
    /// Time at which the transition entered its state.
    timestamp: TimeUnixNanoSec,
    /// Resources held until the next transition.
    usages: SmallVec<[AnalyzedUsage; 1]>,
    /// Original payload retained for application-specific analysis.
    pub data: T,
}

impl<T: TransitionEvent> std::fmt::Debug for AnalyzedTransition<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transition")
            .field("seq", &self.data.sequence())
            .field("timestamp", &self.timestamp)
            .field("state_name", &self.data.name())
            .field("usages", &self.usages)
            .finish_non_exhaustive()
    }
}

impl<T> Timestamp for AnalyzedTransition<T> {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl<T: TransitionEvent> OrderKey for AnalyzedTransition<T> {
    type Key = (TimeUnixNanoSec, u16);

    fn order_key(&self) -> Self::Key {
        // Since the sequence number may wrap, we need to compare both timestamp
        // and sequence number.
        (self.timestamp, self.data.sequence())
    }
}

impl<T: TransitionEvent> Transition for AnalyzedTransition<T> {
    fn name(&self) -> &str {
        self.data.name()
    }

    fn sequence(&self) -> u16 {
        self.data.sequence()
    }

    fn is_final(&self) -> bool {
        self.data.is_final()
    }
}

impl<T> AnalyzedTransition<T> {
    /// Returns resources held for the state ending at the next transition.
    pub fn usages(&self) -> &[AnalyzedUsage] {
        &self.usages
    }
}

struct UsageWithSpan<'a> {
    entity_id: Uuid,
    usage: &'a AnalyzedUsage,
    span: SpanUnixNanoSec,
}

impl<'a> Usage<'a> for UsageWithSpan<'a> {
    fn entity_id(&self) -> Uuid {
        self.entity_id
    }
    fn resource_id(&self) -> Uuid {
        self.usage.resource_id
    }
    fn capacities(&self) -> impl Iterator<Item = &'a CapacityValue> {
        self.usage.capacities.iter()
    }
    fn span(&self) -> SpanUnixNanoSec {
        self.span
    }
}

/// Builds an analyzed Rust-native [`AnalyzedFsm`] from application-specific
/// events.
///
/// This builder orders events by the combination of timestamp and (potentially
/// wrapping) sequence number.
pub struct AnalyzedFsmBuilder<T> {
    id: Uuid,
    transitions: OrderedCollector<AnalyzedTransition<T>>,
    duplicate_order_key: Option<(TimeUnixNanoSec, u16)>,
}

impl<T: TransitionEvent> AnalyzedFsmBuilder<T> {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "fsm id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                transitions: OrderedCollector::default(),
                duplicate_order_key: None,
            })
        }
    }

    /// Return the id of the FSM being built.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Adds one typed transition using its analyzer mapping.
    pub fn push_transition(&mut self, event: Event<T>) {
        let transition = event.data;
        let order_key = (event.timestamp, transition.sequence());
        if self.transitions.push(AnalyzedTransition {
            timestamp: event.timestamp,
            usages: transition.usages(),
            data: transition,
        }) {
            self.duplicate_order_key.get_or_insert(order_key);
        }
    }

    /// Builds an FSM from the collected transitions.
    ///
    /// Missing events are detected only when the observed sequence has no
    /// initial or final transition, or contains an invalid topology edge. Other
    /// missing events may produce inaccurate state spans.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerError::Validation`] if two transitions have the same
    /// timestamp and sequence number, if the first transition is not initial,
    /// or if adjacent transitions violate the FSM topology.
    ///
    /// Returns [`AnalyzerError::IncompleteFsm`] if no final transition was
    /// collected.
    pub fn try_build(self) -> AnalyzerResult<AnalyzedFsm<T>> {
        if let Some((timestamp, sequence)) = self.duplicate_order_key {
            return Err(AnalyzerError::Validation(format!(
                "fsm '{}' (id={}) has multiple transitions with timestamp {timestamp} and sequence {sequence}",
                T::NAME,
                self.id,
            )));
        }
        let transitions: SmallVec<[AnalyzedTransition<T>; INLINE_TRANSITION_CAPACITY]> =
            self.transitions.into_inner().into();
        if let Some(first) = transitions.first()
            && !first.data.is_initial()
        {
            return Err(AnalyzerError::Validation(format!(
                "fsm '{}' (id={}) starts with non-initial transition '{}'",
                T::NAME,
                self.id,
                first.data.name(),
            )));
        }
        if let Some(invalid) = transitions
            .windows(2)
            .find(|transitions| !transitions[0].data.is_valid_next(&transitions[1].data))
        {
            return Err(AnalyzerError::Validation(format!(
                "fsm '{}' (id={}) cannot transition from '{}' to '{}'",
                T::NAME,
                self.id,
                invalid[0].data.name(),
                invalid[1].data.name(),
            )));
        }
        if !transitions
            .last()
            .is_some_and(|transition| transition.data.is_final())
        {
            return Err(AnalyzerError::IncompleteFsm(format!(
                "fsm '{}' (id={}) has no final transition",
                T::NAME,
                self.id
            )));
        }
        Ok(AnalyzedFsm {
            id: self.id,
            transitions,
        })
    }
}

/// An FSM reconstructed from application-specific transition data.
///
/// This type can be constructed through [`AnalyzedFsmBuilder`].
///
/// Application-specific data remains available through [`Self::transitions`].
pub struct AnalyzedFsm<T> {
    id: Uuid,
    transitions: SmallVec<[AnalyzedTransition<T>; INLINE_TRANSITION_CAPACITY]>,
}

impl<T: TransitionEvent> std::fmt::Debug for AnalyzedFsm<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fsm")
            .field("id", &self.id)
            .field("transitions", &self.transitions)
            .finish()
    }
}

impl<T> AnalyzedFsm<T> {
    pub fn transitions(&self) -> &[AnalyzedTransition<T>] {
        &self.transitions
    }
}

impl<T: TransitionEvent> Fsm for AnalyzedFsm<T> {
    type TransitionType = AnalyzedTransition<T>;
    fn len(&self) -> usize {
        self.transitions.len().saturating_sub(1)
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.transitions.get(index)
    }
}

impl<T: TransitionEvent> Entity for AnalyzedFsm<T> {
    fn id(&self) -> Uuid {
        self.id
    }

    fn type_name(&self) -> &str {
        T::NAME
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.transitions
            .first()
            .expect("analyzed FSM must contain at least one transition")
            .timestamp()
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.transitions
            .last()
            .expect("analyzed FSM must contain at least one transition")
            .timestamp()
    }
}

impl<'a, T: TransitionEvent + 'a> FsmUsages<'a> for AnalyzedFsm<T> {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
        self.transitions.windows(2).flat_map(move |window| {
            let name = window[0].name();
            let start = window[0].timestamp();
            let end = window[1].timestamp();
            let span = SpanUnixNanoSec::try_new(start, end).unwrap();
            window[0].usages.iter().map(move |u| {
                (
                    name,
                    UsageWithSpan {
                        entity_id: self.id,
                        usage: u,
                        span,
                    },
                )
            })
        })
    }
}

impl<T: TransitionEvent> Using for AnalyzedFsm<T> {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        self.transitions.windows(2).flat_map(move |window| {
            let start = window[0].timestamp();
            let end = window[1].timestamp();
            let span = SpanUnixNanoSec::try_new(start, end).unwrap();
            window[0].usages.iter().map(move |u| UsageWithSpan {
                entity_id: self.id,
                usage: u,
                span,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct TestTransition {
        sequence: u16,
        is_initial: bool,
        is_final: bool,
    }

    impl EntityEvent for TestTransition {
        const NAME: &'static str = "TestTransition";
    }

    impl TransitionEvent for TestTransition {
        fn name(&self) -> &'static str {
            <Self as EntityEvent>::NAME
        }

        fn sequence(&self) -> u16 {
            self.sequence
        }

        fn is_initial(&self) -> bool {
            self.is_initial
        }

        fn is_final(&self) -> bool {
            self.is_final
        }

        fn is_valid_next(&self, _next: &Self) -> bool {
            !self.is_final
        }
    }

    #[test]
    fn equal_timestamp_transitions_are_ordered_by_sequence() {
        let id = Uuid::from_u128(1);
        let mut builder = AnalyzedFsmBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 1,
                is_initial: false,
                is_final: true,
            },
        ));
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 0,
                is_initial: true,
                is_final: false,
            },
        ));

        let fsm = builder.try_build().unwrap();
        assert_eq!(fsm.type_name(), TestTransition::NAME);
        assert!(fsm.last().unwrap().next_transition().is_final());
        assert_eq!(
            fsm.transitions()
                .iter()
                .map(|transition| transition.data.sequence())
                .collect::<Vec<_>>(),
            [0, 1]
        );
    }

    #[test]
    fn sequence_wrap_is_ordered_by_timestamp() {
        let id = Uuid::from_u128(1);
        let mut builder = AnalyzedFsmBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            101,
            TestTransition {
                sequence: 0,
                is_initial: false,
                is_final: true,
            },
        ));
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: u16::MAX,
                is_initial: true,
                is_final: false,
            },
        ));

        let fsm = builder.try_build().unwrap();
        assert_eq!(
            fsm.transitions()
                .iter()
                .map(|transition| (transition.timestamp(), transition.data.sequence()))
                .collect::<Vec<_>>(),
            [(100, u16::MAX), (101, 0)]
        );
    }

    #[test]
    fn duplicate_transition_order_key_is_rejected() {
        let id = Uuid::from_u128(1);
        let mut builder = AnalyzedFsmBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 0,
                is_initial: true,
                is_final: false,
            },
        ));
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 0,
                is_initial: false,
                is_final: true,
            },
        ));

        assert!(matches!(
            builder.try_build(),
            Err(AnalyzerError::Validation(_))
        ));
    }

    #[test]
    fn invalid_transition_order_is_rejected() {
        let id = Uuid::from_u128(1);
        let mut builder = AnalyzedFsmBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 0,
                is_initial: true,
                is_final: true,
            },
        ));
        builder.push_transition(Event::new(
            id,
            101,
            TestTransition {
                sequence: 1,
                is_initial: false,
                is_final: true,
            },
        ));

        assert!(matches!(
            builder.try_build(),
            Err(AnalyzerError::Validation(_))
        ));
    }

    #[test]
    fn incomplete_fsm_is_rejected() {
        let id = Uuid::from_u128(1);
        let mut builder = AnalyzedFsmBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 0,
                is_initial: true,
                is_final: false,
            },
        ));

        assert!(matches!(
            builder.try_build(),
            Err(AnalyzerError::IncompleteFsm(_))
        ));
    }

    #[test]
    fn non_initial_first_transition_is_rejected() {
        let id = Uuid::from_u128(1);
        let mut builder = AnalyzedFsmBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 1,
                is_initial: false,
                is_final: false,
            },
        ));
        builder.push_transition(Event::new(
            id,
            101,
            TestTransition {
                sequence: 2,
                is_initial: false,
                is_final: true,
            },
        ));

        assert!(matches!(
            builder.try_build(),
            Err(AnalyzerError::Validation(_))
        ));
    }
}
