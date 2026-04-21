// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generic analyzed FSM reconstructed from model-generated events.
//!
//! `FsmEvents<T>` works with any transition enum that implements
//! `TransitionInfo`, providing all the analyzer trait impls (`Entity`, `Fsm`,
//! `FsmUsages`, `Using`, `FsmTypeDeclaration`) without per-FSM boilerplate.

use quent_attributes::Attribute;
use quent_events::Event;
use quent_model::{FsmEvent, ModelBuilder, analyze::TransitionInfo};
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, FsmTypeDecl, FsmTypeDeclaration, FsmUsages, Transition},
    resource::{CapacityValue, Usage, Using},
};

/// A single transition in an analyzed FSM.
pub struct TransitionEvent<T> {
    timestamp: TimeUnixNanoSec,
    state_name: &'static str,
    pub usages: SmallVec<[AnalyzedUsage; 1]>,
    /// The original model transition data.
    pub data: T,
}

impl<T> std::fmt::Debug for TransitionEvent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionEvent")
            .field("timestamp", &self.timestamp)
            .field("state_name", &self.state_name)
            .field("usages", &self.usages)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct AnalyzedUsage {
    pub resource_id: Uuid,
    pub capacities: SmallVec<[CapacityValue; 3]>,
}

impl<T> Timestamp for TransitionEvent<T> {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl<T> Transition for TransitionEvent<T> {
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

/// Builder for reconstructing an `FsmEvents` from model events.
pub struct FsmEventsBuilder<T: TransitionInfo> {
    id: Uuid,
    instance_name: String,
    transitions: TimeOrderedCollector<TransitionEvent<T>>,
}

impl<T: TransitionInfo> FsmEventsBuilder<T> {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "fsm id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                instance_name: String::new(),
                transitions: TimeOrderedCollector::default(),
            })
        }
    }

    /// Return the id of the FSM being built.
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn push(&mut self, event: Event<FsmEvent<T>>) {
        let state = event.data.state;
        let state_name = state.state_name();
        // Capture instance name from the first transition that provides one.
        if self.instance_name.is_empty()
            && let Some(name) = state.instance_name()
        {
            self.instance_name = name.to_owned();
        }
        let extracted = state.usages();
        let usages: SmallVec<[AnalyzedUsage; 1]> = extracted
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
        self.transitions.push(TransitionEvent {
            timestamp: event.timestamp,
            state_name,
            usages,
            data: state,
        });
    }

    pub fn try_build(self) -> AnalyzerResult<FsmEvents<T>> {
        let transitions: SmallVec<[TransitionEvent<T>; 4]> = self.transitions.into_inner().into();
        Ok(FsmEvents {
            id: self.id,
            instance_name: self.instance_name,
            transitions,
        })
    }
}

/// A generic analyzed FSM reconstructed from model-generated events.
///
/// `T` is the transition enum (e.g., `TaskTransition`), which implements
/// `TransitionInfo`. Application-specific data can be accessed via
/// `transitions()` and pattern matching on `T` variants.
///
/// Implements `Entity`, `Fsm`, `FsmUsages`, `Using`, and `FsmTypeDeclaration`.
pub struct FsmEvents<T> {
    id: Uuid,
    instance_name: String,
    transitions: SmallVec<[TransitionEvent<T>; 4]>,
}

impl<T> std::fmt::Debug for FsmEvents<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FsmEvents")
            .field("id", &self.id)
            .field("instance_name", &self.instance_name)
            .field("transitions", &self.transitions)
            .finish()
    }
}

impl<T: TransitionInfo> FsmEvents<T> {
    pub fn transitions(&self) -> &[TransitionEvent<T>] {
        &self.transitions
    }

    /// Access the first transition's data (typically the entry state).
    pub fn first_data(&self) -> Option<&T> {
        self.transitions.first().map(|t| &t.data)
    }
}

impl<T: TransitionInfo> Entity for FsmEvents<T> {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        T::fsm_type_name()
    }
    fn instance_name(&self) -> &str {
        &self.instance_name
    }
}

impl<T: TransitionInfo> Fsm for FsmEvents<T> {
    type TransitionType = TransitionEvent<T>;
    fn len(&self) -> usize {
        self.transitions.len().saturating_sub(1)
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.transitions.get(index)
    }
}

impl<'a, T: TransitionInfo + 'a> FsmUsages<'a> for FsmEvents<T> {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
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
}

impl<T: TransitionInfo> Using for FsmEvents<T> {
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

impl<T: TransitionInfo> FsmTypeDeclaration for FsmEvents<T> {
    fn fsm_type_declaration() -> FsmTypeDecl {
        use crate::fsm::{FsmStateTypeDecl, FsmTransitionDecl};

        let mut builder = ModelBuilder::new("");
        T::collect_model(&mut builder);
        let fsm_def = builder.fsms.into_iter().next().unwrap();

        let states = fsm_def
            .states
            .into_iter()
            .map(|s| FsmStateTypeDecl {
                name: s.name,
                usages: s.usages.iter().map(|u| u.field_name.clone()).collect(),
            })
            .collect();

        let mut transitions = Vec::new();
        for t in &fsm_def.transitions {
            use quent_model::TransitionEndpoint as TE;
            match (&t.from, &t.to) {
                (TE::Entry, TE::State(to)) => {
                    transitions.push(FsmTransitionDecl::Entry(to.clone()));
                }
                (TE::State(from), TE::State(to)) => {
                    transitions.push(FsmTransitionDecl::Transition(from.clone(), to.clone()));
                }
                (TE::State(from), TE::Exit) => {
                    transitions.push(FsmTransitionDecl::Transition(
                        from.clone(),
                        "exit".to_string(),
                    ));
                    transitions.push(FsmTransitionDecl::Exit("exit".to_string()));
                }
                _ => {}
            }
        }

        FsmTypeDecl {
            name: T::fsm_type_name().to_string(),
            states,
            transitions,
        }
    }
}
