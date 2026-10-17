// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_simulator_store::port::PortEvents;

use super::*;

pub type Port = EntityAnalysis<schema::Port>;

impl RefTreeEntity for Port {
    fn parent_id(&self) -> Option<Uuid> {
        self.declaration().map(|event| event.operator_id().target)
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
                .map(|event| event.instance_name().clone()),
            statistics: self
                .statistics()
                .map(|event| query_engine_ui::PortStatistics {
                    custom_statistics: event.custom_attributes().0.clone(),
                }),
        }
    }
}
