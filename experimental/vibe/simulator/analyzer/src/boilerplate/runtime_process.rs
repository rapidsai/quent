// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::AnalyzerError;
use quent_analyzer::context::ContextId;

use super::*;

#[derive(Default)]
pub(crate) struct RuntimeProcessAccumulator {
    native_id: Option<u32>,
    engine_id: Option<Uuid>,
    exited: bool,
}

impl EntityEventAccumulator for RuntimeProcessAccumulator {
    type Event = schema::RuntimeProcessEvent;

    fn push(&mut self, event: Self::Event) {
        match event {
            schema::RuntimeProcessEvent::Started { process, engine_id } => {
                self.native_id = Some(process.native_id);
                self.engine_id = Some(engine_id.target);
            }
            schema::RuntimeProcessEvent::Exit => self.exited = true,
        }
    }
}

pub(crate) struct RuntimeProcess {
    context_id: ContextId,
    entity: AnalyzedEntity<RuntimeProcessAccumulator>,
}

impl RuntimeProcess {
    pub(crate) fn try_from_event(
        context_id: ContextId,
        event: Event<schema::RuntimeProcessEvent>,
    ) -> AnalyzerResult<Self> {
        Ok(Self {
            context_id,
            entity: AnalyzedEntity::try_from_event(event)?,
        })
    }

    pub(crate) fn push(
        &mut self,
        context_id: ContextId,
        event: Event<schema::RuntimeProcessEvent>,
    ) -> AnalyzerResult<()> {
        if self.context_id != context_id {
            return Err(AnalyzerError::Validation(format!(
                "runtime process {} appears in more than one context",
                self.id()
            )));
        }
        self.entity.push(event)
    }

    pub(crate) fn context_id(&self) -> ContextId {
        self.context_id
    }

    pub(crate) fn native_id(&self) -> Option<u32> {
        self.entity.accumulator().native_id
    }
}

impl Entity for RuntimeProcess {
    fn id(&self) -> Uuid {
        self.entity.id()
    }

    fn type_name(&self) -> &str {
        "runtime_process"
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.entity.earliest_timestamp()
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.entity.latest_timestamp()
    }
}

impl RefTreeEntity for RuntimeProcess {
    fn parent_id(&self) -> Option<Uuid> {
        self.entity.accumulator().engine_id
    }
}
