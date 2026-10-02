// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Default)]
pub(crate) struct OperatorAccumulator {
    pub(crate) plan_id: Option<Uuid>,
    pub(crate) parent_operator_ids: Vec<Uuid>,
    pub(crate) instance_name: Option<String>,
    pub(crate) type_name: Option<String>,
    pub(crate) custom_attributes: quent_events::DynamicAttributes,
    pub(crate) statistics: Option<quent_events::DynamicAttributes>,
}

impl EntityEventAccumulator for OperatorAccumulator {
    type Payload = schema::OperatorEvent;

    fn push(&mut self, event: Self::Payload) {
        match event {
            schema::OperatorEvent::Declaration {
                plan_id,
                parent_operator_ids,
                instance_name,
                type_name,
                custom_attributes,
            } => {
                self.plan_id = Some(plan_id.target);
                self.parent_operator_ids = parent_operator_ids
                    .into_iter()
                    .map(|operator| operator.target)
                    .collect();
                self.instance_name = Some(instance_name);
                self.type_name = Some(type_name);
                self.custom_attributes = custom_attributes;
            }
            schema::OperatorEvent::Statistics { custom_attributes } => {
                self.statistics = Some(custom_attributes);
            }
        }
    }
}

#[derive(Debug)]
pub struct Operator {
    entity: AnalyzedEntity<OperatorAccumulator>,
    pub(crate) active_span: Option<SpanUnixNanoSec>,
}

impl Operator {
    pub(crate) fn try_from_event(event: Event<schema::OperatorEvent>) -> AnalyzerResult<Self> {
        Ok(Self {
            entity: AnalyzedEntity::try_from_event(event)?,
            active_span: None,
        })
    }

    pub(crate) fn push(&mut self, event: Event<schema::OperatorEvent>) -> AnalyzerResult<()> {
        self.entity.push(event)
    }

    pub(crate) fn data(&self) -> &OperatorAccumulator {
        self.entity.accumulator()
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
        self.entity.accumulator().plan_id
    }
}

impl OperatorEntity for Operator {
    fn plan_id(&self) -> Option<Uuid> {
        self.data().plan_id
    }

    fn parent_operator_ids(&self) -> impl ExactSizeIterator<Item = Uuid> + '_ {
        self.data().parent_operator_ids.iter().copied()
    }

    fn active_span(&self) -> Option<SpanUnixNanoSec> {
        self.active_span
    }

    fn operator_type_name(&self) -> Option<&str> {
        self.data().type_name.as_deref()
    }

    fn to_ui(&self, epoch: TimeUnixNanoSec) -> query_engine_ui::Operator {
        let data = self.data();
        query_engine_ui::Operator {
            id: self.id(),
            plan_id: data.plan_id,
            parent_operator_ids: data.parent_operator_ids.clone(),
            instance_name: data.instance_name.clone(),
            operator_type_name: data.type_name.clone(),
            custom_attributes: data.custom_attributes.0.clone(),
            statistics: data.statistics.as_ref().map(|statistics| {
                query_engine_ui::OperatorStatistics {
                    custom_statistics: statistics
                        .iter()
                        .map(|attribute| query_engine_ui::OperatorStatistic {
                            value: attribute.clone(),
                            quantity: None,
                        })
                        .collect(),
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use quent_dynamic_attributes::{DynamicAttribute, DynamicStruct};

    #[test]
    fn ui_projection_preserves_producer_order_and_repeated_names() {
        let fields = vec![
            DynamicAttribute::u64("z", 3),
            DynamicAttribute::structure(
                "Work",
                DynamicStruct(vec![
                    DynamicAttribute::u64("second", 2),
                    DynamicAttribute::u64("first", 1),
                ]),
            ),
            DynamicAttribute::u64("z", 4),
        ];
        let operator = Operator::try_from_event(Event::new_now(
            Uuid::from_u128(1),
            schema::OperatorEvent::Statistics {
                custom_attributes: fields.clone().into(),
            },
        ))
        .unwrap();
        let ui = operator.to_ui(TimeUnixNanoSec::default());
        let statistics = ui.statistics.unwrap().custom_statistics;
        assert_eq!(
            statistics
                .iter()
                .map(|s| s.value.key.as_str())
                .collect::<Vec<_>>(),
            vec!["z", "Work", "z"]
        );
        assert_eq!(
            statistics
                .iter()
                .map(|s| s.value.clone())
                .collect::<Vec<_>>(),
            fields
        );
    }
}
