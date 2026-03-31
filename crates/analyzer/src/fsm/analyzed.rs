// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generic analyzed FSM reconstructed from model-generated events.
//!
//! `AnalyzedFsm<T>` works with any transition enum that implements
//! `TransitionInfo`, providing all the analyzer trait impls (`Fsm`,
//! `FsmUsages`, `Using`) without per-FSM boilerplate.

use quent_attributes::Attribute;
use quent_events::Event;
use quent_model::{FsmEvent, analyze::TransitionInfo};
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult,
    fsm::Transition,
    resource::{CapacityValue, Usage},
};

/// A single transition in an analyzed FSM.
#[derive(Debug)]
pub struct AnalyzedTransition<T> {
    timestamp: TimeUnixNanoSec,
    state_name: &'static str,
    pub usages: SmallVec<[AnalyzedUsage; 3]>,
    /// The original model transition data.
    pub data: T,
}

#[derive(Debug)]
pub struct AnalyzedUsage {
    pub resource_id: Uuid,
    pub capacities: SmallVec<[CapacityValue; 3]>,
}

impl<T> Timestamp for AnalyzedTransition<T> {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl<T: std::fmt::Debug> Transition for AnalyzedTransition<T> {
    fn name(&self) -> &str {
        self.state_name
    }

    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        std::iter::empty()
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

/// Builder for reconstructing an `AnalyzedFsm` from model events.
pub struct AnalyzedFsmBuilder<T: TransitionInfo, D> {
    id: Uuid,
    transitions: TimeOrderedCollector<AnalyzedTransition<T>>,
    _deferred: std::marker::PhantomData<D>,
}

impl<T: TransitionInfo, D> AnalyzedFsmBuilder<T, D> {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "fsm id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                transitions: TimeOrderedCollector::default(),
                _deferred: std::marker::PhantomData,
            })
        }
    }

    pub fn push(&mut self, event: Event<FsmEvent<T, D>>) {
        match event.data {
            FsmEvent::Transition { state, .. } => {
                let state_name = state.state_name();
                let extracted = state.usages();
                let usages: SmallVec<[AnalyzedUsage; 3]> = extracted
                    .into_iter()
                    .map(|u| AnalyzedUsage {
                        resource_id: u.resource_id,
                        capacities: u
                            .capacities
                            .into_iter()
                            .map(|c| CapacityValue::new(c.name, c.value.unwrap_or(0)))
                            .collect(),
                    })
                    .collect();
                self.transitions.push(AnalyzedTransition {
                    timestamp: event.timestamp,
                    state_name,
                    usages,
                    data: state,
                });
            }
            FsmEvent::Deferred { .. } => {
                // Deferred events will be merged in future work.
            }
        }
    }

    pub fn try_build(self) -> AnalyzerResult<AnalyzedFsm<T>> {
        let transitions: SmallVec<[AnalyzedTransition<T>; 4]> =
            self.transitions.into_inner().into();
        Ok(AnalyzedFsm {
            id: self.id,
            transitions,
        })
    }
}

/// A generic analyzed FSM reconstructed from model-generated events.
///
/// `T` is the transition enum (e.g., `TaskTransition`), which implements
/// `TransitionInfo`. Application-specific methods can access the original
/// transition data via `transitions()` and match on `T` variants.
#[derive(Debug)]
pub struct AnalyzedFsm<T> {
    id: Uuid,
    transitions: SmallVec<[AnalyzedTransition<T>; 4]>,
}

impl<T: TransitionInfo + std::fmt::Debug> AnalyzedFsm<T> {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn transitions(&self) -> &[AnalyzedTransition<T>] {
        &self.transitions
    }

    /// Access the first transition's data (typically the entry state).
    pub fn first_data(&self) -> Option<&T> {
        self.transitions.first().map(|t| &t.data)
    }
}

impl<T: TransitionInfo + std::fmt::Debug> AnalyzedFsm<T> {
    /// Number of states (transitions minus 1 for the exit).
    pub fn num_states(&self) -> usize {
        self.transitions.len().saturating_sub(1)
    }

    /// Get a transition by index.
    pub fn get_transition(&self, index: usize) -> Option<&AnalyzedTransition<T>> {
        self.transitions.get(index)
    }

    /// Iterate over usage spans (for implementing `FsmUsages` and `Using`).
    pub fn usage_spans(&self) -> impl Iterator<Item = (&str, UsageWithSpan<'_>)> {
        self.transitions.windows(2).flat_map(move |window| {
            let name = window[0].state_name;
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

    /// Flat iteration over all usages (for implementing `Using`).
    pub fn all_usages(&self) -> impl Iterator<Item = UsageWithSpan<'_>> {
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
