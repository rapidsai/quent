// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::entity::EntityEvents;
use quent_analyzer::{AnalyzerResult, Entity, resource::ResourceGroup};
use quent_attributes::Attribute;
use quent_events::Event;
use quent_query_engine_model::port;
use quent_query_engine_ui as ui;
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

/// A Port of an Operator in a Plan DAG.
#[derive(Debug)]
pub struct Port(EntityEvents<port::Port>);

impl Port {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self(EntityEvents::new(id)?))
    }

    pub fn push(&mut self, event: Event<port::PortEvent>) {
        self.0.push(event);
    }

    /// The ID of the operator to which this port belongs.
    pub fn operator_id(&self) -> Option<Uuid> {
        self.0
            .data()
            .declaration
            .as_ref()
            .map(|d| d.operator_id.uuid())
    }

    pub fn to_ui(&self, _epoch: TimeUnixNanoSec) -> ui::Port {
        let d = self.0.data();
        ui::Port {
            id: self.0.id(),
            operator_id: self.operator_id(),
            instance_name: d.declaration.as_ref().map(|d| d.instance_name.clone()),
            statistics: d.statistics.as_ref().map(|s| ui::PortStatistics {
                custom_statistics: s
                    .custom_attributes
                    .iter()
                    .map(|Attribute { key, value }| (key.clone(), value.clone()))
                    .collect(),
            }),
        }
    }
}

impl Entity for Port {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "port"
    }
    fn instance_name(&self) -> &str {
        self.0
            .data()
            .declaration
            .as_ref()
            .map(|d| d.instance_name.as_str())
            .unwrap_or_default()
    }
}

impl ResourceGroup for Port {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.operator_id()
    }
}
