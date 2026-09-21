// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Default)]
pub(crate) struct HostMemoryAccumulator {
    instance_name: Option<String>,
    worker_id: Option<Uuid>,
}

impl EntityEventAccumulator for HostMemoryAccumulator {
    type Event = schema::HostMemoryEvent;

    fn push(&mut self, event: Self::Event) {
        let schema::HostMemoryEvent::Declaration {
            instance_name,
            worker_id,
        } = event;
        self.instance_name = Some(instance_name);
        self.worker_id = Some(worker_id.target);
    }
}

pub(crate) struct HostMemory(AnalyzedEntity<HostMemoryAccumulator>);

impl HostMemory {
    pub(crate) fn try_from_event(event: Event<schema::HostMemoryEvent>) -> AnalyzerResult<Self> {
        Ok(Self(AnalyzedEntity::try_from_event(event)?))
    }

    pub(crate) fn push(&mut self, event: Event<schema::HostMemoryEvent>) -> AnalyzerResult<()> {
        self.0.push(event)
    }

    pub(crate) fn instance_name(&self) -> &str {
        self.0
            .accumulator()
            .instance_name
            .as_deref()
            .expect("host memory must have a declaration event")
    }

    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("host_memory", [CapacityDecl::new_occupancy("bytes")])
    }
}

impl Entity for HostMemory {
    fn id(&self) -> Uuid {
        self.0.id()
    }

    fn type_name(&self) -> &str {
        "host_memory"
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.earliest_timestamp()
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.latest_timestamp()
    }
}

impl Resource for HostMemory {}

impl RefTreeEntity for HostMemory {
    fn parent_id(&self) -> Option<Uuid> {
        self.0.accumulator().worker_id
    }
}
