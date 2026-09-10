// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generic analyzed FSM reconstructed from event data.

pub use quent_dynamic_attributes::DynamicAttribute;
use quent_events::Event;
use quent_time::{OrderKey, OrderedCollector, TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, FsmUsages, Transition},
    resource::{CapacityValue, Usage, Using},
};

/// Provides the data needed to analyze a stored FSM transition.
///
/// This trait applies to the schema-specific transition payload inside an
/// [`Event`]. [`FsmEventsBuilder`] stores the payload with its event timestamp and
/// cached usages in an [`AnalyzedTransition<T>`]. The wrapper implements
/// [`Transition`] for FSM analysis, while [`FsmEvents`] exposes its usages for
/// resource analysis.
///
/// Implementations map schema-specific payloads to data used by analysis APIs,
/// e.g. FSM views and resource timelines. Usages are cached because creating
/// them may allocate. This avoids repeating those allocations across UI requests,
/// such as when requesting a timeline at different zoom levels. Other data is
/// read from the payload when needed.
// TODO(johanpel): Split this adapter into semantic-module-specific traits and
// generate their implementations from the schema.
pub trait AnalyzableTransition: quent_events::EntityEvent {
    /// Returns the entity type name exposed by analysis APIs.
    fn entity_type_name() -> &'static str;

    /// Returns the per-entity ordering key for equal timestamps.
    fn sequence(&self) -> u16;

    /// Returns whether this transition ends the FSM's dynamic lifetime.
    fn is_final(&self) -> bool;

    /// Returns the canonical name of the state entered by this transition.
    fn state_name(&self) -> &'static str;

    /// Returns the FSM instance name when this transition declares it.
    fn instance_name(&self) -> Option<String> {
        None
    }

    /// Returns resources held until the next transition.
    fn usages(&self) -> SmallVec<[AnalyzedUsage; 1]> {
        SmallVec::new()
    }

    /// Returns attributes exposed through [`Transition::attributes`].
    fn dynamic_attributes(&self) -> Vec<DynamicAttribute> {
        Vec::new()
    }
}

/// Stores a transition payload with its resource usages cached for analysis.
///
/// Caching usages during ingestion avoids repeated allocations across UI
/// requests, e.g. timelines requested at different zoom levels. Ordering, state
/// identity, and attributes are read from the payload when needed and require no
/// additional storage.
pub struct AnalyzedTransition<T> {
    /// Time at which the transition entered its state.
    timestamp: TimeUnixNanoSec,
    /// Resources held until the next transition.
    usages: SmallVec<[AnalyzedUsage; 1]>,
    /// Original payload retained for application-specific analysis.
    pub data: T,
}

impl<T: AnalyzableTransition> std::fmt::Debug for AnalyzedTransition<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzedTransition")
            .field("seq", &self.data.sequence())
            .field("timestamp", &self.timestamp)
            .field("state_name", &self.data.state_name())
            .field("usages", &self.usages)
            .finish_non_exhaustive()
    }
}

/// Analyzer representation of one resource usage.
#[derive(Debug)]
pub struct AnalyzedUsage {
    /// Resource used during the state.
    pub resource_id: Uuid,
    /// Capacity values reserved from the resource.
    pub capacities: SmallVec<[CapacityValue; 3]>,
}

impl<T> Timestamp for AnalyzedTransition<T> {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl<T: AnalyzableTransition> OrderKey for AnalyzedTransition<T> {
    type Key = (TimeUnixNanoSec, u16);

    fn order_key(&self) -> Self::Key {
        (self.timestamp, self.data.sequence())
    }
}

impl<T: AnalyzableTransition> Transition for AnalyzedTransition<T> {
    fn name(&self) -> &str {
        self.data.state_name()
    }

    fn attributes(&self) -> Vec<DynamicAttribute> {
        self.data.dynamic_attributes()
    }
}

impl<T> AnalyzedTransition<T> {
    /// Returns resources held for the state ending at the next transition.
    pub fn usages(&self) -> &[AnalyzedUsage] {
        &self.usages
    }
}

pub struct UsageWithSpan<'a> {
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

/// Builder for reconstructing an `FsmEvents` from model events.
pub struct FsmEventsBuilder<T> {
    id: Uuid,
    instance_name: String,
    transitions: OrderedCollector<AnalyzedTransition<T>>,
}

impl<T: AnalyzableTransition> FsmEventsBuilder<T> {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "fsm id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                instance_name: String::new(),
                transitions: OrderedCollector::default(),
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
        // Capture instance name from the first transition that provides one.
        if self.instance_name.is_empty()
            && let Some(name) = transition.instance_name()
        {
            self.instance_name = name;
        }
        self.transitions.push(AnalyzedTransition {
            timestamp: event.timestamp,
            usages: transition.usages(),
            data: transition,
        });
    }

    /// Builds an FSM from the collected transitions.
    ///
    /// Missing intermediate events cannot be detected and may produce inaccurate
    /// state spans.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerError::IncompleteFsm`] if no final transition was
    /// collected.
    pub fn try_build(self) -> AnalyzerResult<FsmEvents<T>> {
        let transitions: SmallVec<[AnalyzedTransition<T>; 4]> =
            self.transitions.into_inner().into();
        if !transitions
            .last()
            .is_some_and(|transition| transition.data.is_final())
        {
            return Err(AnalyzerError::IncompleteFsm(format!(
                "fsm '{}' (id={}) has no final transition",
                T::entity_type_name(),
                self.id
            )));
        }
        Ok(FsmEvents {
            id: self.id,
            instance_name: self.instance_name,
            transitions,
        })
    }
}

/// A generic analyzed FSM reconstructed from model-specific transition data.
///
/// Application-specific data remains available through [`Self::transitions`].
pub struct FsmEvents<T> {
    id: Uuid,
    instance_name: String,
    transitions: SmallVec<[AnalyzedTransition<T>; 4]>,
}

impl<T: AnalyzableTransition> std::fmt::Debug for FsmEvents<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FsmEvents")
            .field("id", &self.id)
            .field("instance_name", &self.instance_name)
            .field("transitions", &self.transitions)
            .finish()
    }
}

impl<T> FsmEvents<T> {
    pub fn transitions(&self) -> &[AnalyzedTransition<T>] {
        &self.transitions
    }

    /// Access the first transition's data (typically the entry state).
    pub fn first_data(&self) -> Option<&T> {
        self.transitions.first().map(|t| &t.data)
    }
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }
}

impl<T: AnalyzableTransition> Fsm for FsmEvents<T> {
    type TransitionType = AnalyzedTransition<T>;
    fn len(&self) -> usize {
        self.transitions.len().saturating_sub(1)
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.transitions.get(index)
    }
}

impl<T: AnalyzableTransition> Entity for FsmEvents<T> {
    fn id(&self) -> Uuid {
        self.id
    }

    fn type_name(&self) -> &str {
        T::entity_type_name()
    }

    fn instance_name(&self) -> &str {
        &self.instance_name
    }
}

impl<'a, T: AnalyzableTransition + 'a> FsmUsages<'a> for FsmEvents<T> {
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

impl<T: AnalyzableTransition> Using for FsmEvents<T> {
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
        is_final: bool,
    }

    impl quent_events::EntityEvent for TestTransition {
        const NAME: &'static str = "TestTransition";
    }

    impl AnalyzableTransition for TestTransition {
        fn entity_type_name() -> &'static str {
            "test"
        }

        fn sequence(&self) -> u16 {
            self.sequence
        }

        fn is_final(&self) -> bool {
            self.is_final
        }

        fn state_name(&self) -> &'static str {
            "test"
        }
    }

    #[test]
    fn equal_timestamp_transitions_are_ordered_by_sequence() {
        let id = Uuid::from_u128(1);
        let mut builder = FsmEventsBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 1,
                is_final: true,
            },
        ));
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 0,
                is_final: false,
            },
        ));

        let fsm = builder.try_build().unwrap();
        assert_eq!(fsm.type_name(), "test");
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
        let mut builder = FsmEventsBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            101,
            TestTransition {
                sequence: 0,
                is_final: true,
            },
        ));
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: u16::MAX,
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
    fn incomplete_fsm_is_rejected() {
        let id = Uuid::from_u128(1);
        let mut builder = FsmEventsBuilder::try_new(id).unwrap();
        builder.push_transition(Event::new(
            id,
            100,
            TestTransition {
                sequence: 0,
                is_final: false,
            },
        ));

        assert!(matches!(
            builder.try_build(),
            Err(AnalyzerError::IncompleteFsm(_))
        ));
    }
}
