// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_simulator_store::operator::OperatorEvents;

use super::*;

#[derive(Debug)]
pub struct Operator {
    events: EntityAnalysis<schema::Operator>,
    active_span: Option<SpanUnixNanoSec>,
}

impl Operator {
    pub(crate) fn new(events: EntityAnalysis<schema::Operator>) -> Self {
        Self {
            events,
            active_span: None,
        }
    }
}

impl Deref for Operator {
    type Target = EntityAnalysis<schema::Operator>;

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl Entity for Operator {
    fn id(&self) -> Uuid {
        self.events.id()
    }

    fn type_name(&self) -> &str {
        self.events.type_name()
    }

    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.events.earliest_timestamp()
    }

    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.events.latest_timestamp()
    }
}

impl RefTreeEntity for Operator {
    fn parent_id(&self) -> Option<Uuid> {
        self.declaration().map(|event| event.plan_id().target)
    }
}

impl OperatorEntity for Operator {
    fn plan_id(&self) -> Option<Uuid> {
        self.parent_id()
    }

    fn parent_operator_ids(&self) -> impl ExactSizeIterator<Item = Uuid> + '_ {
        self.declaration()
            .map(|event| event.parent_operator_ids().as_slice())
            .unwrap_or_default()
            .iter()
            .map(|operator| operator.target)
    }

    fn active_span(&self) -> Option<SpanUnixNanoSec> {
        self.active_span
    }

    fn operator_type_name(&self) -> Option<&str> {
        self.declaration().map(|event| event.type_name().as_str())
    }

    fn to_ui(&self, epoch: TimeUnixNanoSec) -> query_engine_ui::Operator {
        let declaration = self.declaration();
        query_engine_ui::Operator {
            id: self.id(),
            plan_id: self.plan_id(),
            parent_operator_ids: self.parent_operator_ids().collect(),
            instance_name: declaration.map(|event| event.instance_name().clone()),
            operator_type_name: declaration.map(|event| event.type_name().clone()),
            custom_attributes: declaration
                .map(|event| event.custom_attributes().0.clone())
                .unwrap_or_default(),
            statistics: self
                .statistics()
                .map(|event| query_engine_ui::OperatorStatistics {
                    custom_statistics: event
                        .custom_attributes()
                        .iter()
                        .map(|attribute| query_engine_ui::OperatorStatistic {
                            value: attribute.clone(),
                            quantity: None,
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

#[cfg(test)]
mod tests {
    use quent_dynamic_attributes::{DynamicAttribute, DynamicStruct};
    use quent_events::Event;
    use quent_store::entity::native::NativeStore;

    use super::*;

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
        let events = NativeStore::<schema::Operator>::try_from_events([Event::new_now(
            Uuid::from_u128(1),
            schema::OperatorEvent::Statistics {
                custom_attributes: fields.clone().into(),
            },
        )])
        .unwrap();
        let operator = events.map_entities(|events| Operator::new(EntityAnalysis::new(events)));
        let operator = operator.get(Uuid::from_u128(1)).unwrap();
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
