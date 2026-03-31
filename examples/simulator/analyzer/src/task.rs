// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, FsmTypeDecl, FsmTypeDeclaration, FsmUsages, Transition},
    resource::{CapacityValue, Usage, Using},
};
use quent_attributes::Attribute;
use quent_events::Event;
use quent_model::{FsmEvent, ModelComponent, analyze::TransitionInfo};
use quent_simulator_events::task::{
    Queueing, TaskEvent, TaskTransition as ModelTaskTransition,
};
use quent_time::{
    TimeOrderedCollector, TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec, to_secs_relative,
};
use quent_ui::{FiniteStateMachine, FsmTransition, FsmUsage};
use smallvec::SmallVec;
use uuid::Uuid;

// -- Analyzer transition: wraps a model transition with timestamp + extracted usages --

#[derive(Debug)]
struct AnalyzerUsage {
    resource_id: Uuid,
    capacities: SmallVec<[CapacityValue; 3]>,
}

#[derive(Debug)]
pub struct TaskTransition {
    timestamp: TimeUnixNanoSec,
    state_name: &'static str,
    usages: SmallVec<[AnalyzerUsage; 3]>,
    /// The original model transition data, kept for application-specific access.
    data: ModelTaskTransition,
}

impl Timestamp for TaskTransition {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl Transition for TaskTransition {
    fn name(&self) -> &str {
        self.state_name
    }

    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        std::iter::empty()
    }
}

struct UsageWithSpan<'a> {
    task_id: Uuid,
    usage: &'a AnalyzerUsage,
    span: SpanUnixNanoSec,
}

impl<'a> Usage<'a> for UsageWithSpan<'a> {
    fn entity_id(&self) -> Uuid {
        self.task_id
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

// -- Builder: converts model events into analyzer transitions --

pub(crate) struct TaskBuilder {
    id: Uuid,
    transitions: TimeOrderedCollector<TaskTransition>,
}

impl TaskBuilder {
    pub(crate) fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "task id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                transitions: TimeOrderedCollector::default(),
            })
        }
    }

    pub(crate) fn push(&mut self, event: Event<TaskEvent>) {
        match event.data {
            FsmEvent::Transition { state, .. } => {
                // Use TransitionInfo to extract state name and usages generically
                let state_name = state.state_name();
                let extracted = state.usages();
                let usages: SmallVec<[AnalyzerUsage; 3]> = extracted
                    .into_iter()
                    .map(|u| AnalyzerUsage {
                        resource_id: u.resource_id,
                        capacities: u
                            .capacities
                            .into_iter()
                            .map(|c| CapacityValue::new(c.name, c.value.unwrap_or(0)))
                            .collect(),
                    })
                    .collect();
                self.transitions.push(TaskTransition {
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

    pub(crate) fn try_build(self) -> AnalyzerResult<Task> {
        let transitions: SmallVec<[TaskTransition; 4]> = self.transitions.into_inner().into();
        Ok(Task {
            id: self.id,
            transitions,
        })
    }
}

// -- Reconstructed Task FSM --

#[derive(Debug)]
pub struct Task {
    id: Uuid,
    transitions: SmallVec<[TaskTransition; 4]>,
}

impl Task {
    pub fn operator_id(&self) -> Option<Uuid> {
        self.transitions.first().and_then(|t| match &t.data {
            ModelTaskTransition::Queueing(data) => Some(data.operator_id),
            _ => None,
        })
    }

    pub fn active_span(&self) -> Option<SpanUnixNanoSec> {
        let start = self.transitions.get(1)?.timestamp();
        let end = self.transitions.last()?.timestamp();
        SpanUnixNanoSec::try_new(start, end).ok()
    }

    pub fn try_to_ui_fsm(&self, epoch: TimeUnixNanoSec) -> AnalyzerResult<FiniteStateMachine> {
        let transitions = self
            .transitions
            .iter()
            .map(|t| {
                Ok(FsmTransition {
                    name: t.name().to_string(),
                    usages: t
                        .usages
                        .iter()
                        .map(|u| FsmUsage {
                            resource: u.resource_id,
                            capacities: u
                                .capacities
                                .iter()
                                .map(|c| (c.name.to_string(), c.value))
                                .collect(),
                        })
                        .collect(),
                    timestamp: to_secs_relative(t.timestamp(), epoch),
                })
            })
            .collect::<AnalyzerResult<Vec<_>>>()?;

        Ok(FiniteStateMachine {
            id: self.id,
            type_name: self.type_name().to_string(),
            instance_name: self.instance_name().to_string(),
            transitions,
        })
    }
}

impl Entity for Task {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "task"
    }
    fn instance_name(&self) -> &str {
        self.transitions
            .first()
            .and_then(|t| match &t.data {
                ModelTaskTransition::Queueing(Queueing { instance_name, .. }) => {
                    Some(instance_name.as_str())
                }
                _ => None,
            })
            .unwrap_or_default()
    }
}

impl Fsm for Task {
    type TransitionType = TaskTransition;
    fn len(&self) -> usize {
        self.transitions.len().saturating_sub(1)
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.transitions.get(index)
    }
}

impl<'a> FsmUsages<'a> for Task {
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
                        task_id: self.id,
                        usage: u,
                        span,
                    },
                )
            })
        })
    }
}

impl Using for Task {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        self.transitions.windows(2).flat_map(move |window| {
            let start = window[0].timestamp();
            let end = window[1].timestamp();
            let span = SpanUnixNanoSec::try_new(start, end).unwrap();
            window[0].usages.iter().map(move |u| UsageWithSpan {
                task_id: self.id,
                usage: u,
                span,
            })
        })
    }
}

impl FsmTypeDeclaration for Task {
    fn fsm_type_declaration() -> FsmTypeDecl {
        // Collect from the model's ModelComponent metadata.
        let mut builder = quent_model::ModelBuilder::new();
        quent_simulator_model::task::Task::collect(&mut builder);
        let fsm_def = builder.fsms.into_iter().next().unwrap();

        use quent_analyzer::fsm::{FsmStateTypeDecl, FsmTransitionDecl};

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
            match (&t.from, &t.to) {
                (quent_model::TransitionEndpoint::Entry, quent_model::TransitionEndpoint::State(to)) => {
                    transitions.push(FsmTransitionDecl::Entry(to.clone()));
                }
                (quent_model::TransitionEndpoint::State(from), quent_model::TransitionEndpoint::State(to)) => {
                    transitions.push(FsmTransitionDecl::Transition(from.clone(), to.clone()));
                }
                (quent_model::TransitionEndpoint::State(from), quent_model::TransitionEndpoint::Exit) => {
                    transitions.push(FsmTransitionDecl::Transition(from.clone(), "exit".to_string()));
                    transitions.push(FsmTransitionDecl::Exit("exit".to_string()));
                }
                _ => {}
            }
        }

        FsmTypeDecl {
            name: "task".to_string(),
            states,
            transitions,
        }
    }
}
