// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, FsmTypeDecl, FsmTypeDeclaration, FsmUsages, Transition},
    resource::{CapacityValue, Usage, Using},
};
use quent_attributes::Attribute;
use quent_events::Event;
use quent_simulator_events::task::{
    Allocating, Computing, Loading, Queueing, Sending, Spilling, TaskEvent,
};
use quent_time::{
    TimeOrderedCollector, TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec, to_secs_relative,
};
use quent_ui::{FiniteStateMachine, FsmTransition, FsmUsage};
use smallvec::{SmallVec, smallvec};
use uuid::Uuid;

#[derive(Debug)]
pub enum TaskTransitionData {
    Queueing(Queueing),
    Computing(Computing),
    Loading(Loading),
    Allocating(Allocating),
    Spilling(Spilling),
    Sending(Sending),
    Exit,
}

#[derive(Debug)]
pub struct TaskUsage {
    pub resource_id: Uuid,
    pub capacities: SmallVec<[CapacityValue; 3]>,
}

pub struct TaskUsageWithSpan<'a> {
    task_id: Uuid,
    usage: &'a TaskUsage,
    span: SpanUnixNanoSec,
}

impl<'a> Usage<'a> for TaskUsageWithSpan<'a> {
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

#[derive(Debug)]
pub struct TaskTransition {
    timestamp: TimeUnixNanoSec,
    data: TaskTransitionData,
    usages: SmallVec<[TaskUsage; 3]>,
}

impl Timestamp for TaskTransition {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl Transition for TaskTransition {
    fn name(&self) -> &str {
        match &self.data {
            TaskTransitionData::Queueing(_) => "queueing",
            TaskTransitionData::Computing(_) => "computing",
            TaskTransitionData::Loading(_) => "loading",
            TaskTransitionData::Allocating(_) => "allocating",
            TaskTransitionData::Spilling(_) => "spilling",
            TaskTransitionData::Sending(_) => "sending",
            TaskTransitionData::Exit => "exit",
        }
    }

    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        std::iter::empty()
    }
}

fn create_usages(data: &TaskTransitionData) -> SmallVec<[TaskUsage; 3]> {
    match data {
        TaskTransitionData::Queueing(_) => SmallVec::new(),
        TaskTransitionData::Computing(data) => smallvec![
            TaskUsage {
                resource_id: data.use_thread,
                capacities: smallvec![CapacityValue::new("unit", 1)],
            },
            TaskUsage {
                resource_id: data.use_memory,
                capacities: smallvec![CapacityValue::new("bytes", data.use_memory_bytes)],
            },
        ],
        TaskTransitionData::Loading(data) => smallvec![
            TaskUsage {
                resource_id: data.use_thread,
                capacities: smallvec![CapacityValue::new("unit", 1)],
            },
            TaskUsage {
                resource_id: data.use_fs_to_mem,
                capacities: smallvec![CapacityValue::new("bytes", data.use_fs_to_mem_bytes)],
            },
            TaskUsage {
                resource_id: data.use_memory,
                capacities: smallvec![CapacityValue::new("bytes", data.use_memory_bytes)],
            },
        ],
        TaskTransitionData::Allocating(data) => smallvec![TaskUsage {
            resource_id: data.use_thread,
            capacities: smallvec![CapacityValue::new("unit", 1)],
        }],
        TaskTransitionData::Spilling(data) => smallvec![
            TaskUsage {
                resource_id: data.use_thread,
                capacities: smallvec![CapacityValue::new("unit", 1)],
            },
            TaskUsage {
                resource_id: data.use_mem_to_fs,
                capacities: smallvec![CapacityValue::new("bytes", data.use_mem_to_fs_bytes)],
            },
        ],
        TaskTransitionData::Sending(data) => smallvec![
            TaskUsage {
                resource_id: data.use_thread,
                capacities: smallvec![CapacityValue::new("unit", 1)],
            },
            TaskUsage {
                resource_id: data.use_memory,
                capacities: smallvec![CapacityValue::new("bytes", data.use_memory_bytes)],
            },
            TaskUsage {
                resource_id: data.use_link,
                capacities: smallvec![CapacityValue::new("bytes", data.use_link_bytes)],
            },
        ],
        TaskTransitionData::Exit => SmallVec::new(),
    }
}

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
        let data = match event.data {
            TaskEvent::Queueing(data) => TaskTransitionData::Queueing(data),
            TaskEvent::Computing(data) => TaskTransitionData::Computing(data),
            TaskEvent::Allocating(data) => TaskTransitionData::Allocating(data),
            TaskEvent::Loading(data) => TaskTransitionData::Loading(data),
            TaskEvent::Spilling(data) => TaskTransitionData::Spilling(data),
            TaskEvent::Sending(data) => TaskTransitionData::Sending(data),
            TaskEvent::Exit => TaskTransitionData::Exit,
        };
        let usages = create_usages(&data);
        self.transitions.push(TaskTransition {
            timestamp: event.timestamp,
            data,
            usages,
        });
    }

    pub(crate) fn try_build(self) -> AnalyzerResult<Task> {
        let transitions: SmallVec<[TaskTransition; 4]> = self.transitions.into_inner().into();
        // TODO(johanpel): validation goes here
        Ok(Task {
            id: self.id,
            transitions,
        })
    }
}

#[derive(Debug)]
pub struct Task {
    id: Uuid,
    // common case is to at least go through:
    // queueing -> allocating -> computing -> exit
    // hence space for 6 entries by default
    transitions: SmallVec<[TaskTransition; 4]>,
}

impl Task {
    pub fn operator_id(&self) -> Option<Uuid> {
        self.transitions.first().and_then(|t| match &t.data {
            TaskTransitionData::Queueing(data) => Some(data.operator_id),
            _ => None,
        })
    }

    /// Return the span of time in which this task was active, i.e.
    /// non-queueing.
    pub fn active_span(&self) -> Option<SpanUnixNanoSec> {
        let start = self.transitions.get(1)?.timestamp();
        let end = self.transitions.last()?.timestamp();
        SpanUnixNanoSec::try_new(start, end).ok()
    }

    /// Convert this Task to a UI-compatible FSM.
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
                TaskTransitionData::Queueing(data) => Some(data.instance_name.as_str()),
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
                    TaskUsageWithSpan {
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
            window[0].usages.iter().map(move |u| TaskUsageWithSpan {
                task_id: self.id,
                usage: u,
                span,
            })
        })
    }
}

impl FsmTypeDeclaration for Task {
    fn fsm_type_declaration() -> FsmTypeDecl {
        use quent_analyzer::fsm::{FsmStateTypeDecl, FsmTransitionDecl};

        let states = vec![
            FsmStateTypeDecl {
                name: "queueing".to_string(),
                usages: vec![],
            },
            FsmStateTypeDecl {
                name: "computing".to_string(),
                usages: vec!["thread".to_string(), "memory".to_string()],
            },
            FsmStateTypeDecl {
                name: "loading".to_string(),
                usages: vec![
                    "thread".to_string(),
                    "fs_to_mem".to_string(),
                    "memory".to_string(),
                ],
            },
            FsmStateTypeDecl {
                name: "allocating".to_string(),
                usages: vec!["thread".to_string()],
            },
            FsmStateTypeDecl {
                name: "spilling".to_string(),
                usages: vec!["thread".to_string(), "mem_to_fs".to_string()],
            },
            FsmStateTypeDecl {
                name: "sending".to_string(),
                usages: vec![
                    "thread".to_string(),
                    "memory".to_string(),
                    "link".to_string(),
                ],
            },
            FsmStateTypeDecl {
                name: "exit".to_string(),
                usages: vec![],
            },
        ];

        //                          +------------------------+
        //                          |                        v
        // -> queuing -> allocating +----------------+   computing +---> exit
        //                          |                v       ^     v      ^
        //                          +-> spilling -> loading -+   sending -+

        let transitions = vec![
            FsmTransitionDecl::Entry("queueing".to_string()),
            FsmTransitionDecl::Transition("queueing".to_string(), "allocating".to_string()),
            FsmTransitionDecl::Transition("allocating".to_string(), "spilling".to_string()),
            FsmTransitionDecl::Transition("allocating".to_string(), "loading".to_string()),
            FsmTransitionDecl::Transition("allocating".to_string(), "computing".to_string()),
            FsmTransitionDecl::Transition("spilling".to_string(), "loading".to_string()),
            FsmTransitionDecl::Transition("loading".to_string(), "computing".to_string()),
            FsmTransitionDecl::Transition("computing".to_string(), "sending".to_string()),
            FsmTransitionDecl::Transition("computing".to_string(), "exit".to_string()),
            FsmTransitionDecl::Transition("sending".to_string(), "exit".to_string()),
            FsmTransitionDecl::Exit("exit".to_string()),
        ];

        FsmTypeDecl {
            name: "task".to_string(),
            states,
            transitions,
        }
    }
}
