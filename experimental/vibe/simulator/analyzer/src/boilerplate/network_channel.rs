// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Default)]
pub(crate) struct NetworkChannelAccumulator {
    instance_name: Option<String>,
    network_id: Option<Uuid>,
}
impl EntityEventAccumulator for NetworkChannelAccumulator {
    type Event = schema::NetworkChannelEvent;

    fn push(&mut self, event: Self::Event) {
        let schema::NetworkChannelEvent::Declaration {
            instance_name,
            network_id,
        } = event;
        self.instance_name = Some(instance_name);
        self.network_id = Some(network_id.target);
    }
}
pub(crate) struct NetworkChannel(AnalyzedEntity<NetworkChannelAccumulator>);
impl NetworkChannel {
    pub(crate) fn try_from_event(
        event: Event<schema::NetworkChannelEvent>,
    ) -> AnalyzerResult<Self> {
        Ok(Self(AnalyzedEntity::try_from_event(event)?))
    }
    pub(crate) fn push(&mut self, event: Event<schema::NetworkChannelEvent>) -> AnalyzerResult<()> {
        self.0.push(event)
    }
    pub(crate) fn instance_name(&self) -> &str {
        self.0
            .accumulator()
            .instance_name
            .as_deref()
            .expect("network channel must have a declaration event")
    }
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("network_channel", [CapacityDecl::new_rate("bytes")])
    }
}
impl Entity for NetworkChannel {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "network_channel"
    }
    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.earliest_timestamp()
    }
    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.latest_timestamp()
    }
}
impl Resource for NetworkChannel {}
impl RefTreeEntity for NetworkChannel {
    fn parent_id(&self) -> Option<Uuid> {
        self.0.accumulator().network_id
    }
}
