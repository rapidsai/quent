// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Default)]
pub(crate) struct GpuMemoryAccumulator {
    instance_name: Option<String>,
    gpu_id: Option<Uuid>,
}

impl EntityEventAccumulator for GpuMemoryAccumulator {
    type Event = schema::GpuMemoryEvent;

    fn push(&mut self, event: Self::Event) {
        let schema::GpuMemoryEvent::Declaration {
            instance_name,
            gpu_id,
        } = event;
        self.instance_name = Some(instance_name);
        self.gpu_id = Some(gpu_id.target);
    }
}

pub(crate) struct GpuMemory(AnalyzedEntity<GpuMemoryAccumulator>);

impl GpuMemory {
    pub(crate) fn try_from_event(event: Event<schema::GpuMemoryEvent>) -> AnalyzerResult<Self> {
        Ok(Self(AnalyzedEntity::try_from_event(event)?))
    }

    pub(crate) fn push(&mut self, event: Event<schema::GpuMemoryEvent>) -> AnalyzerResult<()> {
        self.0.push(event)
    }

    pub(crate) fn instance_name(&self) -> &str {
        self.0
            .accumulator()
            .instance_name
            .as_deref()
            .expect("GPU memory must have a declaration event")
    }

    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("gpu_memory", [CapacityDecl::new_occupancy("bytes")])
    }
}

impl Entity for GpuMemory {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "gpu_memory"
    }
    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.earliest_timestamp()
    }
    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.latest_timestamp()
    }
}

impl Resource for GpuMemory {}

impl RefTreeEntity for GpuMemory {
    fn parent_id(&self) -> Option<Uuid> {
        self.0.accumulator().gpu_id
    }
}
