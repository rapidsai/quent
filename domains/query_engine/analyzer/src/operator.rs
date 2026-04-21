// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::entity::EntityEvents;
use quent_analyzer::{AnalyzerResult, Entity, resource::ResourceGroup};
use quent_attributes::Attribute;
use quent_events::Event;
use quent_query_engine_model::operator;
use quent_query_engine_ui as ui;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

/// An Operator in a Plan DAG.
#[derive(Debug)]
pub struct Operator {
    inner: EntityEvents<operator::Operator>,
    /// Computed externally from task spans.
    pub active_span: Option<SpanUnixNanoSec>,
}

impl Operator {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self {
            inner: EntityEvents::new(id)?,
            active_span: None,
        })
    }

    pub fn push(&mut self, event: Event<operator::OperatorEvent>) {
        self.inner.push(event);
    }

    /// The ID of the plan this operator belongs to.
    pub fn plan_id(&self) -> Option<Uuid> {
        self.inner
            .data()
            .declaration
            .as_ref()
            .map(|d| d.plan_id.uuid())
    }

    /// The span of time between the first moment an operator started processing
    /// an input, and the latest moment at which an operator finished producing
    /// an output (excluding any potential back-pressure).
    pub fn active_span(&self) -> Option<SpanUnixNanoSec> {
        self.active_span
    }

    pub fn operator_type_name(&self) -> Option<&str> {
        self.inner
            .data()
            .declaration
            .as_ref()
            .map(|d| d.type_name.as_str())
    }

    pub fn to_ui(&self, epoch: TimeUnixNanoSec) -> ui::Operator {
        let d = self.inner.data();

        let custom_attributes = d
            .declaration
            .as_ref()
            .map(|decl| {
                decl.custom_attributes
                    .iter()
                    .map(|Attribute { key, value }| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let statistics = d.statistics.as_ref().map(|s| ui::OperatorStatistics {
            custom_statistics: s
                .custom_attributes
                .iter()
                .map(|Attribute { key, value }| (key.clone(), value.clone()))
                .collect(),
        });

        ui::Operator {
            id: self.inner.id(),
            plan_id: self.plan_id(),
            parent_operator_ids: d
                .declaration
                .as_ref()
                .map(|decl| decl.parent_operator_ids.iter().map(|r| r.uuid()).collect())
                .unwrap_or_default(),
            instance_name: d
                .declaration
                .as_ref()
                .map(|decl| decl.instance_name.clone()),
            operator_type_name: d.declaration.as_ref().map(|decl| decl.type_name.clone()),
            custom_attributes,
            statistics,
            active_span: self
                .active_span()
                .and_then(|span| span.try_to_secs_relative(epoch).ok()),
        }
    }
}

impl Entity for Operator {
    fn id(&self) -> Uuid {
        self.inner.id()
    }
    fn type_name(&self) -> &str {
        "operator"
    }
    fn instance_name(&self) -> &str {
        self.inner
            .data()
            .declaration
            .as_ref()
            .map(|d| d.instance_name.as_str())
            .unwrap_or_default()
    }
}

impl ResourceGroup for Operator {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.plan_id()
    }
}
