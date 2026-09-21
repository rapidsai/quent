// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Default)]
pub(crate) struct TaskExecutorThreadAccumulator {
    instance_name: Option<String>,
    task_executor_id: Option<Uuid>,
}

impl EntityEventAccumulator for TaskExecutorThreadAccumulator {
    type Event = schema::TaskExecutorThreadEvent;

    fn push(&mut self, event: Self::Event) {
        let schema::TaskExecutorThreadEvent::Declaration {
            instance_name,
            task_executor_id,
        } = event;
        self.instance_name = Some(instance_name);
        self.task_executor_id = Some(task_executor_id.target);
    }
}

pub(crate) struct TaskExecutorThread(AnalyzedEntity<TaskExecutorThreadAccumulator>);

impl TaskExecutorThread {
    pub(crate) fn try_from_event(
        event: Event<schema::TaskExecutorThreadEvent>,
    ) -> AnalyzerResult<Self> {
        Ok(Self(AnalyzedEntity::try_from_event(event)?))
    }

    pub(crate) fn push(
        &mut self,
        event: Event<schema::TaskExecutorThreadEvent>,
    ) -> AnalyzerResult<()> {
        self.0.push(event)
    }

    pub(crate) fn instance_name(&self) -> &str {
        self.0
            .accumulator()
            .instance_name
            .as_deref()
            .expect("task executor thread must have a declaration event")
    }

    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::unit("task_executor_thread")
    }
}

impl Entity for TaskExecutorThread {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "task_executor_thread"
    }
    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.earliest_timestamp()
    }
    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.latest_timestamp()
    }
}

impl Resource for TaskExecutorThread {}

impl RefTreeEntity for TaskExecutorThread {
    fn parent_id(&self) -> Option<Uuid> {
        self.0.accumulator().task_executor_id
    }
}
