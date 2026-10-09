// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::operator::{OperatorEventStorage, OperatorEvents};

use super::*;

#[derive(Debug)]
pub struct Operator {
    entity: StoredEntity<schema::Operator, OperatorEventStorage>,
    pub(crate) active_span: Option<SpanUnixNanoSec>,
}

impl Operator {
    pub(crate) fn try_from_sequence(
        sequence: quent_store::entity::sequence::EventSequence<schema::Operator>,
    ) -> AnalyzerResult<Self> {
        Ok(Self {
            entity: StoredEntity::try_from_sequence(sequence)?,
            active_span: None,
        })
    }
}

impl Entity for Operator {
    fn id(&self) -> Uuid {
        self.entity.id()
    }
    fn type_name(&self) -> &str {
        self.entity.type_name()
    }
    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.entity.earliest_timestamp()
    }
    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.entity.latest_timestamp()
    }
}

impl RefTreeEntity for Operator {
    fn parent_id(&self) -> Option<Uuid> {
        self.plan_id()
    }
}

impl OperatorEntity for Operator {
    fn plan_id(&self) -> Option<Uuid> {
        self.entity
            .declaration()
            .map(|event| event.data.plan_id.target)
    }

    fn parent_operator_ids(&self) -> impl ExactSizeIterator<Item = Uuid> + '_ {
        self.entity
            .declaration()
            .map_or(&[][..], |event| event.data.parent_operator_ids.as_slice())
            .iter()
            .map(|operator| operator.target)
    }

    fn active_span(&self) -> Option<SpanUnixNanoSec> {
        self.active_span
    }

    fn operator_type_name(&self) -> Option<&str> {
        self.entity
            .declaration()
            .map(|event| event.data.type_name.as_str())
    }

    fn to_ui(&self, epoch: TimeUnixNanoSec) -> query_engine_ui::Operator {
        let declaration = self.entity.declaration().map(|event| &event.data);
        query_engine_ui::Operator {
            id: self.id(),
            plan_id: self.plan_id(),
            parent_operator_ids: self.parent_operator_ids().collect(),
            instance_name: declaration.map(|data| data.instance_name.clone()),
            operator_type_name: declaration.map(|data| data.type_name.clone()),
            custom_attributes: declaration
                .into_iter()
                .flat_map(|data| data.custom_attributes.iter())
                .map(|attribute| (attribute.key.clone(), attribute.value.clone()))
                .collect(),
            statistics: self
                .entity
                .statistics()
                .map(|event| query_engine_ui::OperatorStatistics {
                    custom_statistics: event
                        .data
                        .custom_attributes
                        .iter()
                        .map(|attribute| {
                            (
                                attribute.key.clone(),
                                query_engine_ui::OperatorStatistic {
                                    value: attribute.value.clone(),
                                    quantity: None,
                                },
                            )
                        })
                        .collect(),
                }),
            active_span: self
                .active_span
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
