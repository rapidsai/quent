use std::collections::HashMap;

use quent_entities::{
    fsm::Fsm,
    resource::CapacityType,
    timeline::{ResourceTimelineBinned, ResourceTimelineBinnedByState},
};
use quent_time::bin::BinnedSpan;
use uuid::Uuid;

use crate::{
    Result,
    entities::Entities,
    timeline::binned::{BinnedTimelineAggregator, NamedAggregator},
};

pub mod binned;

pub fn make_resource_timeline_bin_aggregated(
    entities: &Entities,
    resource_id: Uuid,
    config: BinnedSpan,
) -> Result<ResourceTimelineBinned> {
    let resource = entities.resource(resource_id)?;
    let resource_type = entities.resource_type(&resource.type_name)?;

    let mut aggregator = NamedAggregator::new(config);

    for (span, capacity) in entities
        .iter_dynamic_fsms(resource_id)
        // Flatten into spans and states with potential uses of this resource
        .flat_map(|fsm| fsm.state_spans())
        // Flatten into spans into usages.
        .flat_map(|state_span| {
            let span = state_span.span;
            state_span.state.uses.iter().map(move |usage| (span, usage))
        })
        // Filter uses that aren't targeting this resource
        .filter(|(_, usage)| usage.resource == resource_id)
        // Flatten into various capacities of this resource
        .flat_map(|(span, usage)| usage.capacities.iter().map(move |amount| (span, amount)))
    {
        if let Some(value) = capacity.value {
            let capacity_type = resource_type.try_capacity(&capacity.name)?.kind;
            let value = match capacity_type {
                CapacityType::Occupancy => value as f64,
                CapacityType::Rate => value as f64 / span.duration() as f64,
            };
            aggregator.try_push(span, (value, capacity.name.as_str()))?
        }
    }

    Ok(ResourceTimelineBinned {
        config: config.try_to_secs_relative(entities.span.start())?,
        capacities_values: aggregator
            .try_finish()?
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    })
}

pub fn make_resource_timeline_state_and_bin_aggregated(
    entities: &Entities,
    resource_id: Uuid,
    config: BinnedSpan,
    fsm_type_name: impl Into<String>,
) -> Result<ResourceTimelineBinnedByState> {
    let fsm_type_name = fsm_type_name.into();
    let resource = entities.resource(resource_id)?;
    let resource_type = entities.resource_type(&resource.type_name)?;

    let mut binners: HashMap<&str, NamedAggregator> = HashMap::new();

    for (span, capacity, state_name) in entities
        .iter_dynamic_fsms(resource_id)
        // Filter out FSMs that don't have this type name
        .filter(|fsm| fsm.type_name() == fsm_type_name)
        // Flatten into spans and states with potential uses of this resource
        .flat_map(|fsm| fsm.state_spans())
        // Flatten into spans into usages.
        .flat_map(|state_span| {
            state_span
                .state
                .uses
                .iter()
                .map(move |usage| (state_span.span, usage, &state_span.state.name))
        })
        // Filter states of which their usages aren't targeting this resource
        .filter(|(_, usage, _)| usage.resource == resource_id)
        // Flatten into various capacities of this resource
        .flat_map(|(span, usage, state_name)| {
            usage
                .capacities
                .iter()
                .map(move |amount| (span, amount, state_name))
        })
    {
        if let Some(value) = capacity.value {
            let capacity_kind = resource_type.try_capacity(&capacity.name)?.kind;
            let value = match capacity_kind {
                CapacityType::Occupancy => value as f64,
                CapacityType::Rate => value as f64 / span.duration() as f64,
            };
            binners
                .entry(capacity.name.as_str())
                .or_insert_with(|| NamedAggregator::new(config))
                .try_push(span, (value, state_name))?
        }
    }

    let capacities_states_values = binners
        .into_iter()
        .map(|(k, v)| {
            v.try_finish().map(|values| {
                (
                    k.to_string(),
                    values
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v))
                        .collect(),
                )
            })
        })
        .collect::<Result<HashMap<_, _>>>()?;

    Ok(ResourceTimelineBinnedByState {
        config: config.try_to_secs_relative(entities.span.start())?,
        capacities_states_values,
    })
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use crate::{entities::Entities, timeline::make_resource_timeline_bin_aggregated};

    use super::*;
    use quent_entities::{
        EntityRef, fsm,
        resource::{CapacityDecl, CapacityValue, Resource, ResourceTypeDecl, Use},
    };
    use quent_events::{Event, EventData, engine, resource};
    use quent_time::{SpanNanoSec, bin::BinnedSpan};

    fn engine_events(id: Uuid) -> [Event<EventData>; 4] {
        [
            Event::new(
                id,
                0,
                EventData::Engine(engine::EngineEvent::Init(engine::Init {
                    implementation: None,
                    name: None,
                })),
            ),
            Event::new(
                id,
                0,
                EventData::Engine(engine::EngineEvent::Operating(engine::Operating {})),
            ),
            Event::new(
                id,
                1000,
                EventData::Engine(engine::EngineEvent::Finalizing(engine::Finalizing {})),
            ),
            Event::new(
                id,
                1000,
                EventData::Engine(engine::EngineEvent::Exit(engine::Exit {})),
            ),
        ]
    }

    fn memory_events(engine_id: Uuid, resource_id: Uuid) -> [Event<EventData>; 4] {
        [
            Event::new(
                resource_id,
                0,
                EventData::Resource(resource::ResourceEvent::Memory(
                    resource::memory::MemoryEvent::Init(resource::memory::Init {
                        resource: resource::Resource {
                            instance_name: "test_inst".to_string(),
                            type_name: "test".to_string(),
                            scope: resource::Scope::Engine(engine_id),
                        },
                    }),
                )),
            ),
            Event::new(
                resource_id,
                0,
                EventData::Resource(resource::ResourceEvent::Memory(
                    resource::memory::MemoryEvent::Operating(resource::memory::Operating {
                        capacity_bytes: 1000,
                    }),
                )),
            ),
            Event::new(
                resource_id,
                1000,
                EventData::Resource(resource::ResourceEvent::Memory(
                    resource::memory::MemoryEvent::Finalizing(resource::memory::Finalizing {
                        unreclaimed_bytes: 0,
                    }),
                )),
            ),
            Event::new(
                resource_id,
                1000,
                EventData::Resource(resource::ResourceEvent::Memory(
                    resource::memory::MemoryEvent::Exit(resource::memory::Exit {}),
                )),
            ),
        ]
    }

    #[test]
    fn test_resource_timeline_aggregated() {
        let engine_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();

        // Feed some events
        let events = engine_events(engine_id)
            .into_iter()
            .chain(memory_events(engine_id, resource_id));

        let mut entities = Entities::try_new(engine_id, events).unwrap();

        // Produce triangle-ish memory utilization using 4 FSMs
        for i in 0..4 {
            let fsm = Uuid::now_v7();
            let start = i * 100;
            let end = 1000 - i * 100;
            entities.fsms.insert(
                fsm,
                fsm::DynamicFsm::try_new(
                    fsm,
                    "test",
                    None,
                    [
                        fsm::DynamicState {
                            name: "using".into(),
                            uses: vec![Use::new(resource_id, [CapacityValue::new("bytes", 250)])],
                            timestamp: start,
                            attributes: vec![],
                            relations: vec![],
                        },
                        fsm::DynamicState {
                            name: "exit".into(),
                            uses: vec![],
                            timestamp: end,
                            attributes: vec![],
                            relations: vec![],
                        },
                    ],
                )
                .unwrap(),
            );
        }

        // Sanity check
        assert_eq!(
            entities
                .iter_dynamic_fsms(resource_id)
                .collect::<Vec<_>>()
                .len(),
            4
        );

        let config = BinnedSpan::try_new(
            SpanNanoSec::try_new(0, 1000).unwrap(),
            NonZero::try_from(10).unwrap(),
        )
        .unwrap();

        let timeline =
            make_resource_timeline_bin_aggregated(&entities, resource_id, config).unwrap();

        // Config shouldn't be modified.
        assert_eq!(timeline.config, config.try_to_secs_relative(0).unwrap());

        // We should have bin datapoints for the "bytes" capacity.
        assert!(timeline.capacities_values.contains_key("bytes"));

        let values = timeline.capacities_values.get("bytes").unwrap();

        // Check whether the "trianglish" utilization is correct after aggregation:
        assert_eq!(
            values[..],
            [
                // FSMs using 250 capacity each:
                //  1|   1,2| 1,2,3|1,2,3,4|1,2,3,4|1,2,3,4|1,2,3,4| 1,2,3|   1,2|     1|
                250.0, 500.0, 750.0, 1000.0, 1000.0, 1000.0, 1000.0, 750.0, 500.0, 250.0,
            ],
        );
    }

    #[test]
    fn test_resource_timeline_aggregated_multi_capacity() {
        let engine_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();

        let events = engine_events(engine_id).into_iter();
        let mut entities = Entities::try_new(engine_id, events).unwrap();

        // Add a resource with 2 capacities.
        entities.resource_types.insert(
            "test".to_string(),
            ResourceTypeDecl::new(
                "test",
                &[
                    CapacityDecl::new_occupancy("a"),
                    CapacityDecl::new_occupancy("b"),
                ] as &[CapacityDecl],
            ),
        );
        entities.resources.insert(
            resource_id,
            Resource {
                id: resource_id,
                instance_name: Some("test".into()),
                type_name: "test".into(),
                scope: Some(EntityRef::Engine(engine_id)),
                state_sequence: vec![],
            },
        );

        // Spawn 2 FSMs using both capacities
        for i in 0..2 {
            let fsm = Uuid::now_v7();
            entities.fsms.insert(
                fsm,
                fsm::DynamicFsm::try_new(
                    fsm,
                    "test",
                    None,
                    [
                        fsm::DynamicState {
                            name: "using".into(),
                            uses: vec![Use::new(
                                resource_id,
                                &[CapacityValue::new("a", 250), CapacityValue::new("b", 1)]
                                    as &[CapacityValue],
                            )],
                            timestamp: i * 250,
                            attributes: vec![],
                            relations: vec![],
                        },
                        fsm::DynamicState {
                            name: "exit".into(),
                            uses: vec![],
                            timestamp: 1000 - i * 250,
                            attributes: vec![],
                            relations: vec![],
                        },
                    ],
                )
                .unwrap(),
            );
        }

        // Sanity check
        assert_eq!(
            entities
                .iter_dynamic_fsms(resource_id)
                .collect::<Vec<_>>()
                .len(),
            2
        );

        let config = BinnedSpan::try_new(
            SpanNanoSec::try_new(0, 1000).unwrap(),
            NonZero::try_from(10).unwrap(),
        )
        .unwrap();

        let timeline =
            make_resource_timeline_bin_aggregated(&entities, resource_id, config).unwrap();

        assert_eq!(timeline.config, config.try_to_secs_relative(0).unwrap());
        assert!(timeline.capacities_values.contains_key("a"));
        assert!(timeline.capacities_values.contains_key("b"));

        let a = timeline.capacities_values.get("a").unwrap();
        let b = timeline.capacities_values.get("b").unwrap();

        assert_eq!(
            a[..],
            [
                // 3 FSMs increasing 250 of "a" at intervals of 250.
                // Bin start - end -> Util FSM 1 + Util FSM 2 + Util FSM 3
                // 000 - 100 -> 250
                // 100 - 200 -> ...
                // 200 - 300 -> 250 + (300 - 250) / 100 * 250 = 375
                // 300 - 400 -> 250 + (400 - 300) / 100 * 250 = 500
                // 400 - 500 -> ...
                // 500 - 600 -> ...
                // 600 - 700 -> ...
                // 700 - 800 -> 250 + (800 - 750) / 100 * 250 = 375
                // 800 - 900 -> 250
                // 900- 1000 -> ...
                250.0, 250.0, 375.0, 500.0, 500.0, 500.0, 500.0, 375.0, 250.0, 250.0
            ],
        );
        assert_eq!(b[..], [1.0, 1.0, 1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.0, 1.0],);
    }

    #[test]
    fn test_resource_timeline_aggregated_multi_state() {
        let engine_id = Uuid::now_v7();
        let resource_id = Uuid::now_v7();

        // Feed some events
        let events = engine_events(engine_id)
            .into_iter()
            .chain(memory_events(engine_id, resource_id));

        let mut entities = Entities::try_new(engine_id, events).unwrap();

        // Produce triangle-ish memory utilization using 4 FSMs with 2 usage states
        for i in 0..4 {
            let fsm = Uuid::now_v7();
            let start = 100 * i;
            let end = 1000 - i * 100;
            entities.fsms.insert(
                fsm,
                fsm::DynamicFsm::try_new(
                    fsm,
                    "test",
                    None,
                    [
                        fsm::DynamicState {
                            name: "state_a".into(),
                            uses: vec![Use::new(resource_id, [CapacityValue::new("bytes", 250)])],
                            timestamp: start,
                            attributes: vec![],
                            relations: vec![],
                        },
                        fsm::DynamicState {
                            name: "state_b".into(),
                            uses: vec![Use::new(resource_id, [CapacityValue::new("bytes", 42)])],
                            timestamp: start + (end - start) / 2,
                            attributes: vec![],
                            relations: vec![],
                        },
                        fsm::DynamicState {
                            name: "exit".into(),
                            uses: vec![],
                            timestamp: end,
                            attributes: vec![],
                            relations: vec![],
                        },
                    ],
                )
                .unwrap(),
            );
        }

        // Sanity check
        assert_eq!(
            entities
                .iter_dynamic_fsms(resource_id)
                .collect::<Vec<_>>()
                .len(),
            4
        );

        let config = BinnedSpan::try_new(
            SpanNanoSec::try_new(0, 1000).unwrap(),
            NonZero::try_from(10).unwrap(),
        )
        .unwrap();

        let timeline =
            make_resource_timeline_state_and_bin_aggregated(&entities, resource_id, config, "test")
                .unwrap();

        // Config shouldn't be modified.
        assert_eq!(timeline.config, config.try_to_secs_relative(0).unwrap());

        // For each capacity, we should have values per state
        assert!(timeline.capacities_states_values.contains_key("bytes"));
        let bytes = timeline.capacities_states_values.get("bytes").unwrap();
        assert!(bytes.contains_key("state_a"));
        assert!(bytes.contains_key("state_b"));

        let state_a_bytes = bytes.get("state_a").unwrap();
        let state_b_bytes = bytes.get("state_b").unwrap();

        // Check whether the "trianglish" utilization is correct after aggregation:
        assert_eq!(
            state_a_bytes[..],
            [
                // all four "a" states ramp up with 250 steps and end halfway the entire timeline
                1.0 * 250.0,
                2.0 * 250.0,
                3.0 * 250.0,
                4.0 * 250.0,
                4.0 * 250.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ],
        );
        assert_eq!(
            state_b_bytes[..],
            // all four "b" states start with 42 util halfway the tlime and then ramp down
            [
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                42.0 * 4.0,
                42.0 * 4.0,
                42.0 * 3.0,
                42.0 * 2.0,
                42.0 * 1.0,
            ],
        );
    }
}
