// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::entity::EntityEvents;
use quent_analyzer::{AnalyzerResult, Entity, resource::ResourceGroup};
use quent_dynamic_attributes::DynamicAttribute;
use quent_events::Event;
use quent_query_engine_model::operator;
use quent_query_engine_ui as ui;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

use crate::{OperatorEntity, OperatorEntityMut};

/// An event-backed operator in a plan DAG.
#[derive(Debug)]
pub struct Operator {
    inner: EntityEvents<operator::Operator>,
    active_span: Option<SpanUnixNanoSec>,
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
}

impl OperatorEntity for Operator {
    fn plan_id(&self) -> Option<Uuid> {
        self.inner
            .data()
            .declaration
            .as_ref()
            .map(|d| d.plan_id.uuid())
    }

    fn parent_operator_ids(&self) -> impl ExactSizeIterator<Item = Uuid> + '_ {
        self.inner
            .data()
            .declaration
            .as_ref()
            .map(|declaration| declaration.parent_operator_ids.as_slice())
            .unwrap_or_default()
            .iter()
            .map(|parent| parent.uuid())
    }

    fn active_span(&self) -> Option<SpanUnixNanoSec> {
        self.active_span
    }

    fn operator_type_name(&self) -> Option<&str> {
        self.inner
            .data()
            .declaration
            .as_ref()
            .map(|d| d.type_name.as_str())
    }

    fn to_ui(&self, epoch: TimeUnixNanoSec) -> ui::Operator {
        let d = self.inner.data();

        let custom_attributes = d
            .declaration
            .as_ref()
            .map(|decl| {
                decl.custom_attributes
                    .iter()
                    .map(|DynamicAttribute { key, value }| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let statistics = d.statistics.as_ref().map(|s| ui::OperatorStatistics {
            custom_statistics: s
                .custom_attributes
                .iter()
                .map(|DynamicAttribute { key, value }| {
                    (
                        key.clone(),
                        ui::OperatorStatistic {
                            value: value.clone(),
                            quantity: None,
                        },
                    )
                })
                .collect(),
        });

        ui::Operator {
            id: self.inner.id(),
            plan_id: self.plan_id(),
            parent_operator_ids: self.parent_operator_ids().collect(),
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

impl OperatorEntityMut for Operator {
    fn extend_active_span(&mut self, span: SpanUnixNanoSec) {
        self.active_span = Some(match self.active_span {
            Some(existing) => existing.extend(&span),
            None => span,
        });
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
