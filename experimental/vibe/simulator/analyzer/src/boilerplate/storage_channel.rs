// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Default)]
pub(crate) struct StorageChannelAccumulator {
    instance_name: Option<String>,
    worker_id: Option<Uuid>,
}

impl EntityEventAccumulator for StorageChannelAccumulator {
    type Event = schema::StorageChannelEvent;

    fn push(&mut self, event: Self::Event) {
        let schema::StorageChannelEvent::Declaration {
            instance_name,
            worker_id,
        } = event;
        self.instance_name = Some(instance_name);
        self.worker_id = Some(worker_id.target);
    }
}

pub(crate) struct StorageChannel(AnalyzedEntity<StorageChannelAccumulator>);

impl StorageChannel {
    pub(crate) fn try_from_event(
        event: Event<schema::StorageChannelEvent>,
    ) -> AnalyzerResult<Self> {
        Ok(Self(AnalyzedEntity::try_from_event(event)?))
    }
    pub(crate) fn push(&mut self, event: Event<schema::StorageChannelEvent>) -> AnalyzerResult<()> {
        self.0.push(event)
    }
    pub(crate) fn instance_name(&self) -> &str {
        self.0
            .accumulator()
            .instance_name
            .as_deref()
            .expect("storage channel must have a declaration event")
    }
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("storage_channel", [CapacityDecl::new_rate("bytes")])
    }
}

impl Entity for StorageChannel {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "storage_channel"
    }
    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.earliest_timestamp()
    }
    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.latest_timestamp()
    }
}
impl Resource for StorageChannel {}
impl RefTreeEntity for StorageChannel {
    fn parent_id(&self) -> Option<Uuid> {
        self.0.accumulator().worker_id
    }
}
