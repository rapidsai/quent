// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Default)]
pub(crate) struct StorageAccumulator {
    instance_name: Option<String>,
    worker_id: Option<Uuid>,
}

impl EntityEventAccumulator for StorageAccumulator {
    type Event = schema::StorageEvent;

    fn push(&mut self, event: Self::Event) {
        let schema::StorageEvent::Declaration {
            instance_name,
            worker_id,
        } = event;
        self.instance_name = Some(instance_name);
        self.worker_id = Some(worker_id.target);
    }
}

pub(crate) struct Storage(AnalyzedEntity<StorageAccumulator>);

impl Storage {
    pub(crate) fn try_from_event(event: Event<schema::StorageEvent>) -> AnalyzerResult<Self> {
        Ok(Self(AnalyzedEntity::try_from_event(event)?))
    }

    pub(crate) fn push(&mut self, event: Event<schema::StorageEvent>) -> AnalyzerResult<()> {
        self.0.push(event)
    }

    pub(crate) fn instance_name(&self) -> &str {
        self.0
            .accumulator()
            .instance_name
            .as_deref()
            .expect("storage must have a declaration event")
    }

    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("storage", [CapacityDecl::new_occupancy("bytes")])
    }
}

impl Entity for Storage {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "storage"
    }
    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.earliest_timestamp()
    }
    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.0.latest_timestamp()
    }
}

impl Resource for Storage {}

impl RefTreeEntity for Storage {
    fn parent_id(&self) -> Option<Uuid> {
        self.0.accumulator().worker_id
    }
}
