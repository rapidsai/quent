// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Default)]
pub(crate) struct PcieChannelAccumulator {
    instance_name: Option<String>,
    gpu_id: Option<Uuid>,
}
impl EntityEventAccumulator for PcieChannelAccumulator {
    type Event = schema::PcieChannelEvent;

    fn push(&mut self, event: Self::Event) {
        let schema::PcieChannelEvent::Declaration {
            instance_name,
            gpu_id,
        } = event;
        self.instance_name = Some(instance_name);
        self.gpu_id = Some(gpu_id.target);
    }
}
pub(crate) struct PcieChannel(AnalyzedEntity<PcieChannelAccumulator>);
impl PcieChannel {
    pub(crate) fn try_from_event(event: Event<schema::PcieChannelEvent>) -> AnalyzerResult<Self> {
        Ok(Self(AnalyzedEntity::try_from_event(event)?))
    }
    pub(crate) fn push(&mut self, event: Event<schema::PcieChannelEvent>) -> AnalyzerResult<()> {
        self.0.push(event)
    }
    pub(crate) fn instance_name(&self) -> &str {
        self.0
            .accumulator()
            .instance_name
            .as_deref()
            .expect("PCIe channel must have a declaration event")
    }
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("pcie_channel", [CapacityDecl::new_rate("bytes")])
    }
}
impl Entity for PcieChannel {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "pcie_channel"
    }
    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.earliest_timestamp()
    }
    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.latest_timestamp()
    }
}
impl Resource for PcieChannel {}
impl RefTreeEntity for PcieChannel {
    fn parent_id(&self) -> Option<Uuid> {
        self.0.accumulator().gpu_id
    }
}
