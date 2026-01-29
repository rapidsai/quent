use std::collections::{HashMap, HashSet};

use quent_entities::{
    fsm::{Fsm, State, StateSpan},
    resource::{CapacityValue, ResourceTypeDecl},
    timeline::{ResourceTimelineBinned, ResourceTimelineBinnedByState},
};
use quent_time::{SpanNanoSec, bin::BinnedSpan};
use uuid::Uuid;

use crate::{
    AnalyzerResult,
    entities::Entities,
    resource_tree::ResourceTree,
    timeline::binned::{BinnedTimelineAggregator, NamedAggregator},
};

pub mod binned;

/// Given an iterator over FSMs, extract all capacity usages and their
/// associated spans.
fn iter_capacity_usages<'a, F>(
    fsms: impl Iterator<Item = &'a F>,
    resource_filter: impl Fn(Uuid) -> bool,
) -> impl Iterator<Item = (SpanNanoSec, &'a CapacityValue)>
where
    F: Fsm + 'a,
{
    // Flatten FSMs into states with their associated spans.
    fsms.flat_map(|fsm| fsm.state_spans())
        // Flatten states into all usages
        .flat_map(|StateSpan { span, state }| state.uses().map(move |usage| (span, usage)))
        // Filter usages with the resource filter
        .filter(move |(_, usage)| resource_filter(usage.resource))
        // Flatten usages into individual capacities
        .flat_map(|(span, usage)| usage.capacities.iter().map(move |amount| (span, amount)))
}

/// Given an iterator over FSMs, extract all capacity usages and their
/// associated spans and state name.
fn iter_capacity_usages_with_state<'a, F>(
    fsms: impl Iterator<Item = &'a F>,
    resource_filter: impl Fn(Uuid) -> bool,
) -> impl Iterator<Item = (SpanNanoSec, &'a CapacityValue, &'a str)>
where
    F: Fsm + 'a,
{
    // Flatten FSMs into states with their associated spans.
    fsms.flat_map(|fsm| fsm.state_spans())
        // Flatten states into usages, keeping span and state name.
        .flat_map(|StateSpan { span, state }| {
            state.uses().map(move |usage| (span, usage, state.name()))
        })
        // Filter usages with the resource filter
        .filter(move |(_, usage, _)| resource_filter(usage.resource))
        // Flatten usages into individual capacities
        .flat_map(|(span, usage, state_name)| {
            usage
                .capacities
                .iter()
                .map(move |amount| (span, amount, state_name))
        })
}

/// Calculate a value to bin-aggregate depending on the [`CapacityType`].
fn convert_capacity(
    span: SpanNanoSec,
    capacity_value: &CapacityValue,
    resource_type: &ResourceTypeDecl,
) -> AnalyzerResult<f64> {
    let capacity_type = resource_type.try_capacity(&capacity_value.name)?.kind;
    Ok(capacity_type.reinterpret_capacity_value(capacity_value.value.unwrap_or_default(), span))
}

pub fn make_resource_timeline_bin_aggregated(
    entities: &Entities,
    resource_id: Uuid,
    config: BinnedSpan,
) -> AnalyzerResult<ResourceTimelineBinned> {
    let resource = entities.resource(resource_id)?;
    let resource_type = entities.resource_type(&resource.type_name)?;

    let mut aggregator = NamedAggregator::new(config);

    for (span, capacity) in iter_capacity_usages(entities.dynamic_fsms_using(resource_id), |r| {
        r == resource_id
    }) {
        if capacity.value.is_some() {
            let value = convert_capacity(span, capacity, resource_type)?;
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
    fsm_type_name: &str,
    config: BinnedSpan,
) -> AnalyzerResult<ResourceTimelineBinnedByState> {
    let resource = entities.resource(resource_id)?;
    let resource_type = entities.resource_type(&resource.type_name)?;

    let mut binners: HashMap<&str, NamedAggregator> = HashMap::new();

    for (span, capacity, state_name) in iter_capacity_usages_with_state(
        entities
            .dynamic_fsms_using(resource_id)
            .filter(|fsm| fsm.type_name() == fsm_type_name),
        |r| r == resource_id,
    ) {
        if capacity.value.is_some() {
            let value = convert_capacity(span, capacity, resource_type)?;
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
        .collect::<AnalyzerResult<HashMap<_, _>>>()?;

    Ok(ResourceTimelineBinnedByState {
        config: config.try_to_secs_relative(entities.span.start())?,
        capacities_states_values,
    })
}

pub fn make_resource_group_timeline_bin_aggregated(
    entities: &Entities,
    resource_group_id: Uuid,
    resource_type_name: &str,
    config: BinnedSpan,
) -> AnalyzerResult<ResourceTimelineBinned> {
    // Construct the tree with the provided resource group as root.
    let tree = ResourceTree::try_new(entities, resource_group_id)?;
    // Look up the resource type
    let resource_type = entities.resource_type(resource_type_name)?;
    // Iterate over all leaf reasources, only keeping those of the requested type.
    let resources = tree
        .iter_leaves()
        .filter_map(|resource_id| {
            entities
                .resource(resource_id)
                .map(|resource| (resource.type_name == resource_type_name).then_some(resource.id))
                .transpose()
        })
        .collect::<AnalyzerResult<HashSet<_>>>()?;

    let mut aggregator = NamedAggregator::new(config);

    for (span, capacity) in
        iter_capacity_usages(entities.dynamic_fsms_using_any_of(&resources), |resource| {
            resources.contains(&resource)
        })
    {
        aggregator.try_push(
            span,
            (
                convert_capacity(span, capacity, resource_type)?,
                capacity.name.as_str(),
            ),
        )?
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

pub fn make_resource_group_timeline_state_and_bin_aggregated(
    entities: &Entities,
    resource_group_id: Uuid,
    resource_type_name: &str,
    fsm_type_name: &str,
    config: BinnedSpan,
) -> AnalyzerResult<ResourceTimelineBinnedByState> {
    let tree = ResourceTree::try_new(entities, resource_group_id)?;
    let resource_type = entities.resource_type(resource_type_name)?;
    let resources = tree
        .iter_leaves()
        .filter_map(|resource_id| {
            entities
                .resource(resource_id)
                .map(|resource| (resource.type_name == resource_type_name).then_some(resource.id))
                .transpose()
        })
        .collect::<AnalyzerResult<HashSet<_>>>()?;

    let mut binners: HashMap<&str, NamedAggregator> = HashMap::new();

    for (span, capacity, state_name) in iter_capacity_usages_with_state(
        entities
            .dynamic_fsms_using_any_of(&resources)
            .filter(|fsm| fsm.type_name() == fsm_type_name),
        |resource_id| resources.contains(&resource_id),
    ) {
        if capacity.value.is_some() {
            let value = convert_capacity(span, capacity, resource_type)?;
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
        .collect::<AnalyzerResult<HashMap<_, _>>>()?;

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
        fsm,
        resource::{CapacityDecl, CapacityValue, Resource, ResourceTypeDecl, Use},
    };
    use quent_events::{
        Event, EventData, engine,
        resource::{self, ResourceGroup},
    };
    use quent_time::{SpanNanoSec, bin::BinnedSpan};

    fn engine_events(id: Uuid) -> [Event<EventData>; 5] {
        [
            Event::new(
                Uuid::from_u64_pair(0, 1),
                0,
                EventData::ResourceGroup(ResourceGroup {
                    instance_name: "test".into(),
                    parent_group_id: None,
                }),
            ),
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

    fn memory_events(resource_id: Uuid) -> [Event<EventData>; 4] {
        [
            Event::new(
                resource_id,
                0,
                EventData::Resource(resource::ResourceEvent::Memory(
                    resource::memory::MemoryEvent::Init(resource::memory::Init {
                        resource: resource::Resource {
                            instance_name: "test_inst".to_string(),
                            type_name: "test".to_string(),
                            parent_group_id: Uuid::from_u64_pair(0, 1),
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
            .chain(memory_events(resource_id));

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
                .dynamic_fsms_using(resource_id)
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
    fn test_resource_group_timeline_aggregated() {
        // Declare and use two memory resources of the same type
        let engine_id = Uuid::now_v7();
        let resource_a_id = Uuid::now_v7();
        let resource_b_id = Uuid::now_v7();

        // Feed some events
        let events = engine_events(engine_id)
            .into_iter()
            .chain(memory_events(resource_a_id))
            .chain(memory_events(resource_b_id));

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
                            uses: vec![Use::new(
                                if i % 2 == 0 {
                                    resource_a_id
                                } else {
                                    resource_b_id
                                },
                                [CapacityValue::new("bytes", 250)],
                            )],
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
                .dynamic_fsms_using(resource_a_id)
                .collect::<Vec<_>>()
                .len(),
            2
        );
        assert_eq!(
            entities
                .dynamic_fsms_using(resource_b_id)
                .collect::<Vec<_>>()
                .len(),
            2
        );

        dbg!(&entities);

        let timeline = make_resource_group_timeline_bin_aggregated(
            &entities,
            Uuid::from_u64_pair(0, 1), // root resource group
            "test",
            BinnedSpan::try_new(
                SpanNanoSec::try_new(0, 1000).unwrap(),
                NonZero::try_from(10).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

        let values = timeline.capacities_values.get("bytes").unwrap();

        // Aggregated, it should produce the same result as
        // test_resource_timeline_aggregated
        assert_eq!(
            values[..],
            [
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
                parent_id: Uuid::from_u64_pair(0, 1),
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
                .dynamic_fsms_using(resource_id)
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
            .chain(memory_events(resource_id));

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
                .dynamic_fsms_using(resource_id)
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
            make_resource_timeline_state_and_bin_aggregated(&entities, resource_id, "test", config)
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

    #[test]
    fn test_resource_timeline_group_aggregated_multi_state() {
        let engine_id = Uuid::now_v7();
        let resource_a_id = Uuid::now_v7();
        let resource_b_id = Uuid::now_v7();

        // Feed some events
        let events = engine_events(engine_id)
            .into_iter()
            .chain(memory_events(resource_a_id))
            .chain(memory_events(resource_b_id));

        let mut entities = Entities::try_new(engine_id, events).unwrap();

        // Produce the same utilization as previous test but spread it out across two leaf resources.
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
                            uses: vec![Use::new(
                                if i % 2 == 0 {
                                    resource_a_id
                                } else {
                                    resource_b_id
                                },
                                [CapacityValue::new("bytes", 250)],
                            )],
                            timestamp: start,
                            attributes: vec![],
                            relations: vec![],
                        },
                        fsm::DynamicState {
                            name: "state_b".into(),
                            uses: vec![Use::new(
                                if i % 2 == 0 {
                                    resource_a_id
                                } else {
                                    resource_b_id
                                },
                                [CapacityValue::new("bytes", 42)],
                            )],
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

        let config = BinnedSpan::try_new(
            SpanNanoSec::try_new(0, 1000).unwrap(),
            NonZero::try_from(10).unwrap(),
        )
        .unwrap();
        let timeline = make_resource_group_timeline_state_and_bin_aggregated(
            &entities,
            Uuid::from_u64_pair(0, 1),
            "test",
            "test",
            config,
        )
        .unwrap();

        let bytes = timeline.capacities_states_values.get("bytes").unwrap();
        let state_a_bytes = bytes.get("state_a").unwrap();
        let state_b_bytes = bytes.get("state_b").unwrap();

        // If we aggregate over both resources we should get the same answer as
        // in test_resource_timeline_aggregated_multi_state
        assert_eq!(
            state_a_bytes[..],
            [
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
