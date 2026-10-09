// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::worker::{WorkerEventStorage, WorkerEvents};

use super::*;

pub type Worker = StoredEntity<schema::Worker, WorkerEventStorage>;

impl RefTreeEntity for Worker {
    fn parent_id(&self) -> Option<Uuid> {
        self.init().map(|event| event.data.parent_engine_id.target)
    }
}

impl WorkerEntity for Worker {
    fn to_ui(&self, _epoch: TimeUnixNanoSec) -> query_engine_ui::Worker {
        query_engine_ui::Worker {
            id: self.id(),
            parent_engine_id: self.parent_id(),
            instance_name: self.init().map(|event| event.data.instance_name.clone()),
            start_unix_ns: Some(self.earliest_timestamp()),
            end_unix_ns: self.exit().map(|_| self.latest_timestamp()),
        }
    }
}
