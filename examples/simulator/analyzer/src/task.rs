// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{
    AnalyzerResult, Entity,
    fsm::{Fsm, FsmTypeDecl, FsmTypeDeclaration, FsmUsages, Transition,
          analyzed::{AnalyzedFsm, AnalyzedFsmBuilder, AnalyzedTransition}},
    resource::{Usage, Using},
};
use quent_model::ModelComponent;
use quent_simulator_events::task::{
    Queueing, TaskDeferred, TaskTransition as ModelTaskTransition,
};
use quent_time::{TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec, to_secs_relative};
use quent_ui::{FiniteStateMachine, FsmTransition, FsmUsage};
use uuid::Uuid;

// -- Type aliases --

pub type TaskBuilder = AnalyzedFsmBuilder<ModelTaskTransition, TaskDeferred>;

// -- Task: thin wrapper over AnalyzedFsm with application-specific methods --

#[derive(Debug)]
pub struct Task {
    inner: AnalyzedFsm<ModelTaskTransition>,
}

impl Task {
    pub fn from_builder(builder: TaskBuilder) -> AnalyzerResult<Self> {
        Ok(Self {
            inner: builder.try_build()?,
        })
    }

    pub fn operator_id(&self) -> Option<Uuid> {
        self.inner.first_data().and_then(|t| match t {
            ModelTaskTransition::Queueing(data) => Some(data.operator_id),
            _ => None,
        })
    }

    pub fn active_span(&self) -> Option<SpanUnixNanoSec> {
        let start = self.inner.get_transition(1)?.timestamp();
        let end = self.inner.transitions().last()?.timestamp();
        SpanUnixNanoSec::try_new(start, end).ok()
    }

    pub fn try_to_ui_fsm(&self, epoch: TimeUnixNanoSec) -> AnalyzerResult<FiniteStateMachine> {
        let transitions = self
            .inner
            .transitions()
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
            id: self.inner.id(),
            type_name: self.type_name().to_string(),
            instance_name: self.instance_name().to_string(),
            transitions,
        })
    }
}

impl Entity for Task {
    fn id(&self) -> Uuid {
        self.inner.id()
    }
    fn type_name(&self) -> &str {
        "task"
    }
    fn instance_name(&self) -> &str {
        self.inner
            .first_data()
            .and_then(|t| match t {
                ModelTaskTransition::Queueing(Queueing { instance_name, .. }) => {
                    Some(instance_name.as_str())
                }
                _ => None,
            })
            .unwrap_or_default()
    }
}

impl Fsm for Task {
    type TransitionType = AnalyzedTransition<ModelTaskTransition>;
    fn len(&self) -> usize {
        self.inner.num_states()
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.inner.get_transition(index)
    }
}

impl<'a> FsmUsages<'a> for Task {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
        self.inner.usage_spans()
    }
}

impl Using for Task {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        self.inner.all_usages()
    }
}

impl FsmTypeDeclaration for Task {
    fn fsm_type_declaration() -> FsmTypeDecl {
        use quent_analyzer::fsm::{FsmStateTypeDecl, FsmTransitionDecl};

        let mut builder = quent_model::ModelBuilder::new();
        quent_simulator_model::task::Task::collect(&mut builder);
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
