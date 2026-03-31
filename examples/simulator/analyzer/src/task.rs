// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM analysis types.
//!
//! With `AnalyzedFsm<T>` providing all generic trait impls (`Entity`, `Fsm`,
//! `FsmUsages`, `Using`, `FsmTypeDeclaration`), the task analyzer is just
//! type aliases plus application-specific helper methods.

use quent_analyzer::{
    AnalyzerResult, Entity,
    fsm::{Transition, analyzed::{AnalyzedFsm, AnalyzedFsmBuilder}},
};
use quent_simulator_events::task::{
    TaskDeferred, TaskTransition as ModelTaskTransition,
};
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec, Timestamp, to_secs_relative};
use quent_ui::{FiniteStateMachine, FsmTransition, FsmUsage};
use uuid::Uuid;

/// The reconstructed Task FSM.
pub type Task = AnalyzedFsm<ModelTaskTransition>;

/// Builder for Task FSMs.
pub type TaskBuilder = AnalyzedFsmBuilder<ModelTaskTransition, TaskDeferred>;

/// Application-specific methods on the Task FSM.
pub trait TaskExt {
    fn operator_id(&self) -> Option<Uuid>;
    fn active_span(&self) -> Option<SpanUnixNanoSec>;
    fn try_to_ui_fsm(&self, epoch: TimeUnixNanoSec) -> AnalyzerResult<FiniteStateMachine>;
}

impl TaskExt for Task {
    fn operator_id(&self) -> Option<Uuid> {
        self.first_data().and_then(|t| match t {
            ModelTaskTransition::Queueing(data) => Some(data.operator_id),
            _ => None,
        })
    }

    fn active_span(&self) -> Option<SpanUnixNanoSec> {
        let start = self.transitions().get(1)?.timestamp();
        let end = self.transitions().last()?.timestamp();
        SpanUnixNanoSec::try_new(start, end).ok()
    }

    fn try_to_ui_fsm(&self, epoch: TimeUnixNanoSec) -> AnalyzerResult<FiniteStateMachine> {
        let transitions = self
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
            id: self.id(),
            type_name: self.type_name().to_string(),
            instance_name: self.instance_name().to_string(),
            transitions,
        })
    }
}
