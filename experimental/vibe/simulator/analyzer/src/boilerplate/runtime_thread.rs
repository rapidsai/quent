// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::AnalyzerError;
use quent_analyzer::context::ContextId;

use super::*;

#[derive(Default)]
pub(crate) struct RuntimeThreadAccumulator {
    native_id: Option<u64>,
    process_id: Option<Uuid>,
    logical_thread_id: Option<Uuid>,
    exited: bool,
}

impl EntityEventAccumulator for RuntimeThreadAccumulator {
    type Event = schema::RuntimeThreadEvent;

    fn push(&mut self, event: Self::Event) {
        match event {
            schema::RuntimeThreadEvent::Started {
                thread,
                process_id,
                logical_thread_id,
            } => {
                self.native_id = Some(thread.native_id);
                self.process_id = Some(process_id.target);
                self.logical_thread_id = Some(logical_thread_id.target);
            }
            schema::RuntimeThreadEvent::Exit => self.exited = true,
        }
    }
}

pub(crate) struct RuntimeThread {
    context_id: ContextId,
    entity: AnalyzedEntity<RuntimeThreadAccumulator>,
}

impl RuntimeThread {
    pub(crate) fn try_from_event(
        context_id: ContextId,
        event: Event<schema::RuntimeThreadEvent>,
    ) -> AnalyzerResult<Self> {
        Ok(Self {
            context_id,
            entity: AnalyzedEntity::try_from_event(event)?,
        })
    }

    pub(crate) fn push(
        &mut self,
        context_id: ContextId,
        event: Event<schema::RuntimeThreadEvent>,
    ) -> AnalyzerResult<()> {
        if self.context_id != context_id {
            return Err(AnalyzerError::Validation(format!(
                "runtime thread {} appears in more than one context",
                self.id()
            )));
        }
        self.entity.push(event)
    }

    pub(crate) fn context_id(&self) -> ContextId {
        self.context_id
    }

    pub(crate) fn native_id(&self) -> Option<u64> {
        self.entity.accumulator().native_id
    }

    pub(crate) fn process_id(&self) -> Option<Uuid> {
        self.entity.accumulator().process_id
    }

    pub(crate) fn logical_thread_id(&self) -> Option<Uuid> {
        self.entity.accumulator().logical_thread_id
    }

    pub(crate) fn contains(&self, timestamp: TimeUnixNanoSec) -> bool {
        self.earliest_timestamp() <= timestamp
            && (!self.entity.accumulator().exited || timestamp <= self.latest_timestamp())
    }
}

impl Entity for RuntimeThread {
    fn id(&self) -> Uuid {
        self.entity.id()
    }

    fn type_name(&self) -> &str {
        "runtime_thread"
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.entity.earliest_timestamp()
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.entity.latest_timestamp()
    }
}

impl RefTreeEntity for RuntimeThread {
    fn parent_id(&self) -> Option<Uuid> {
        self.entity.accumulator().process_id
    }
}
