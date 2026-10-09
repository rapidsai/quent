// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::port::{PortEventStorage, PortEvents};

use super::*;

pub type Port = StoredEntity<schema::Port, PortEventStorage>;

impl RefTreeEntity for Port {
    fn parent_id(&self) -> Option<Uuid> {
        self.declaration()
            .map(|event| event.data.operator_id.target)
    }
}

impl PortEntity for Port {
    fn operator_id(&self) -> Option<Uuid> {
        self.parent_id()
    }

    fn to_ui(&self, _epoch: TimeUnixNanoSec) -> query_engine_ui::Port {
        query_engine_ui::Port {
            id: self.id(),
            operator_id: self.operator_id(),
            instance_name: self
                .declaration()
                .map(|event| event.data.instance_name.clone()),
            statistics: self
                .statistics()
                .map(|event| query_engine_ui::PortStatistics {
                    custom_statistics: event
                        .data
                        .custom_attributes
                        .iter()
                        .map(|attribute| (attribute.key.clone(), attribute.value.clone()))
                        .collect(),
                }),
        }
    }
}
