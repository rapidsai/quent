// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM analysis types.

use quent_analyzer::{
    AnalyzerResult, Entity,
    fsm::{
        Fsm, FsmStateTypeDecl, FsmTransitionDecl, FsmTypeDecl, FsmTypeDeclaration, FsmUsages,
        Transition,
        events::{AnalyzedTransition, FsmEvents, FsmEventsBuilder},
    },
    resource::{Usage, Using},
};
use quent_simulator_store as schema;
use quent_time::{TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec, to_secs_relative};
use quent_ui::{FiniteStateMachine, FsmTransition, FsmUsage};
use uuid::Uuid;

fn declaration() -> FsmTypeDecl {
    let state = |name: &str, usages: &[&str]| FsmStateTypeDecl {
        name: name.to_owned(),
        usages: usages.iter().map(|usage| (*usage).to_owned()).collect(),
    };
    FsmTypeDecl {
        name: "task".to_owned(),
        states: vec![
            state("queueing", &[]),
            state("allocating", &["use_thread"]),
            state(
                "loading",
                &[
                    "use_thread",
                    "use_storage_channel",
                    "use_pcie_channel",
                    "use_host_memory",
                    "use_gpu_memory",
                ],
            ),
            state(
                "computing",
                &["use_thread", "use_host_memory", "use_gpu_memory"],
            ),
            state("spilling", &["use_thread", "use_storage_channel"]),
            state("sending", &["use_thread", "use_network_channel"]),
            state("exit", &[]),
        ],
        transitions: vec![
            FsmTransitionDecl::Entry("queueing".to_owned()),
            FsmTransitionDecl::Transition("queueing".to_owned(), "allocating".to_owned()),
            FsmTransitionDecl::Transition("allocating".to_owned(), "loading".to_owned()),
            FsmTransitionDecl::Transition("allocating".to_owned(), "computing".to_owned()),
            FsmTransitionDecl::Transition("loading".to_owned(), "loading".to_owned()),
            FsmTransitionDecl::Transition("loading".to_owned(), "computing".to_owned()),
            FsmTransitionDecl::Transition("computing".to_owned(), "spilling".to_owned()),
            FsmTransitionDecl::Transition("computing".to_owned(), "sending".to_owned()),
            FsmTransitionDecl::Transition("computing".to_owned(), "exit".to_owned()),
            FsmTransitionDecl::Transition("spilling".to_owned(), "allocating".to_owned()),
            FsmTransitionDecl::Transition("sending".to_owned(), "queueing".to_owned()),
            FsmTransitionDecl::Transition("sending".to_owned(), "exit".to_owned()),
            FsmTransitionDecl::Exit("exit".to_owned()),
        ],
    }
}

/// The reconstructed Task FSM.
#[derive(Debug)]
pub struct Task(FsmEvents<schema::TaskEvent>);

impl Task {
    pub(crate) fn from_builder(builder: TaskBuilder) -> AnalyzerResult<Self> {
        Ok(Self(builder.try_build()?))
    }

    pub fn transitions(&self) -> &[AnalyzedTransition<schema::TaskEvent>] {
        self.0.transitions()
    }

    fn first_data(&self) -> Option<&schema::TaskEvent> {
        self.0.first_data()
    }
}

/// Builder for Task FSMs.
pub type TaskBuilder = FsmEventsBuilder<schema::TaskEvent>;

impl Entity for Task {
    fn id(&self) -> Uuid {
        self.0.id()
    }

    fn type_name(&self) -> &str {
        "task"
    }

    fn instance_name(&self) -> &str {
        self.0.instance_name()
    }
}

impl Fsm for Task {
    type TransitionType = AnalyzedTransition<schema::TaskEvent>;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.0.transition(index)
    }
}

impl<'a> FsmUsages<'a> for Task {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
        self.0.usages_with_state_names()
    }
}

impl Using for Task {
    fn usages(&self) -> impl Iterator<Item = impl Usage<'_>> {
        self.0.usages()
    }
}

impl FsmTypeDeclaration for Task {
    fn fsm_type_declaration() -> FsmTypeDecl {
        declaration()
    }
}

/// Application-specific methods on the Task FSM.
pub trait TaskExt {
    fn operator_id(&self) -> Option<Uuid>;
    fn active_span(&self) -> Option<SpanUnixNanoSec>;
    fn try_to_ui_fsm(&self, epoch: TimeUnixNanoSec) -> AnalyzerResult<FiniteStateMachine>;
}

impl TaskExt for Task {
    fn operator_id(&self) -> Option<Uuid> {
        self.first_data().and_then(|t| match t {
            schema::TaskEvent::Queueing { operator_id, .. } => Some(operator_id.target),
            _ => None,
        })
    }

    fn active_span(&self) -> Option<SpanUnixNanoSec> {
        let start = self.transitions().get(1)?.timestamp();
        let end = self.transitions().last()?.timestamp();
        SpanUnixNanoSec::try_new(start, end).ok()
    }

    fn try_to_ui_fsm(&self, epoch: TimeUnixNanoSec) -> AnalyzerResult<FiniteStateMachine> {
        let raw = self.transitions();
        let transitions = raw
            .iter()
            .enumerate()
            .map(|(i, t)| {
                // Derive the processing rate from input_bytes over the span.
                let mut derived_attributes = vec![];
                if let schema::TaskEvent::Computing { input_bytes, .. } = &t.data
                    && let Some(next) = raw.get(i + 1)
                {
                    let span_secs = (next.timestamp() - t.timestamp()) as f64 / 1e9;
                    if span_secs > 0.0 {
                        derived_attributes.push(quent_dynamic_attributes::DynamicAttribute::f64(
                            "bytes_per_sec",
                            *input_bytes as f64 / span_secs,
                        ));
                    }
                }
                Ok(FsmTransition {
                    name: t.name().to_string(),
                    usages: t
                        .usages()
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
                    attributes: t.attributes(),
                    derived_attributes,
                })
            })
            .collect::<AnalyzerResult<Vec<_>>>()?;

        Ok(FiniteStateMachine {
            id: self.id(),
            type_name: self.type_name().to_string(),
            instance_name: self.instance_name().to_string(),
            transitions,
        })
    }
}
