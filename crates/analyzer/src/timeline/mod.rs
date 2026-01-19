use std::collections::HashMap;

use quent_entities::{
    EntityRef,
    fsm::Fsm,
    timeline::{ResourceTimeline, ResourceTimelineBinned, ResourceTimelineUse},
};
use quent_time::{Span, bin::BinnedSpan};
use uuid::Uuid;

use crate::{
    Result,
    entities::Entities,
    error::Error,
    timeline::binned::{BinnedTimelineAggregator, NumericPrimitiveBinnedTimeline},
};

pub mod binned;

fn check_entity_exists_or_error(entities: &Entities, resource_id: Uuid) -> Result<()> {
    match entities.get_entity_ref_from_id(resource_id) {
        Some(EntityRef::Resource(_)) => Ok(()),
        Some(entity_ref) => Err(Error::Logic(format!(
            "ID {resource_id} is an entity but it is not a resource ({entity_ref:?})"
        ))),
        None => Err(Error::InvalidId(resource_id)),
    }
}

// TODO(johanpel): maybe move everything into a single crate and make traits for this
// TODO(johanpel); combine / chain this with Related::use_relations
pub fn make_fsm_resource_timeline_uses(
    fsm: &Fsm,
    resource_id: Uuid,
) -> impl Iterator<Item = ResourceTimelineUse> {
    let usage_states = fsm.state_sequence.iter().enumerate().flat_map({
        move |(index, state)| {
            state
                .uses
                .iter()
                .filter(move |u| u.resource == resource_id)
                .map(move |u| (index, u))
        }
    });

    usage_states.map(|(state_index, usage)| ResourceTimelineUse {
        span: fsm.state_span(state_index).unwrap().span,
        amounts: usage.capacities.clone(),
        entity: EntityRef::CustomFsm(fsm.id),
    })
}

pub fn make_resource_timeline_for_resource(
    entities: &Entities,
    resource_id: Uuid,
) -> Result<ResourceTimeline> {
    // TODO(johanpel): could be supplied with an entity name and a state name filter

    check_entity_exists_or_error(entities, resource_id)?;

    // TODO(johanpel): not unwrap
    let span = Span::try_new(
        entities.engine.timestamps.init.unwrap(),
        entities.engine.timestamps.exit.unwrap(),
    )?;

    let uses = entities
        .iter_use_relations()
        .filter_map(|(user, resource)| (Uuid::from(resource) == resource_id).then_some(user))
        .filter_map(|user| match user {
            quent_entities::EntityRef::CustomFsm(uuid) => {
                Some(entities.custom_fsms.get(&uuid).unwrap())
            }
            _ => None,
        })
        .flat_map(|fsm| make_fsm_resource_timeline_uses(fsm, resource_id))
        .collect::<Vec<_>>();

    Ok(ResourceTimeline { span, uses })
}

pub fn make_resource_timeline_bin_aggregated(
    entities: &Entities,
    resource_id: Uuid,
    config: BinnedSpan,
) -> Result<ResourceTimelineBinned> {
    // Sanity checks
    check_entity_exists_or_error(entities, resource_id)?;

    let resource = entities.resources.get(&resource_id).unwrap();

    // Prepare a place to put binners. Since we're going to bin capacity uses
    // which MUST be primitive numeric values per specification, we will be able
    // to use NumericPrimitiveBinnedTimeline here.
    let mut binners: HashMap<&str, NumericPrimitiveBinnedTimeline> = HashMap::new();

    for (span, capacity) in entities
        // Iterate over all entities that use resources
        .iter_use_relations()
        // Filter out entities that don't use our target resource
        .filter_map(|(user, resource)| (Uuid::from(resource) == resource_id).then_some(user))
        // Filter out anything that's not an FSM for now :tm:
        .filter_map(|user| match user {
            quent_entities::EntityRef::CustomFsm(uuid) => {
                Some(entities.custom_fsms.get(&uuid).unwrap())
            }
            _ => None,
        })
        // Flatten into spans and states with potential uses of this resource
        .flat_map(|fsm| {
            (0..fsm.state_sequence.len() - 1).map(|index| fsm.state_span(index).unwrap())
        })
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
        if let Some(value) = &capacity.value {
            let capacity_type =
                resource
                    .capacities
                    .get(&capacity.key)
                    .ok_or(Error::Logic(format!(
                        "Use of undeclared resource capacity {}. Possible capacities only include: {:?}",
                        capacity.key,
                        resource.capacities.keys()
                    )))?.kind;
            binners
                .entry(capacity.key.as_str())
                .or_insert_with(|| NumericPrimitiveBinnedTimeline::new(config))
                .try_push(span, (value.clone(), capacity_type))?
        }
    }

    Ok(ResourceTimelineBinned {
        config,
        capacity_values: binners
            .into_iter()
            .map(|(k, v)| v.try_finish().map(|values| (k.to_string(), values)))
            .collect::<Result<HashMap<_, _>>>()?,
    })
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use crate::{entities::Entities, timeline::make_resource_timeline_bin_aggregated};

    use super::*;
    use quent_attributes::{Attribute, Value};
    use quent_entities::{
        fsm,
        resource::{CapacityDecl, Resource, Use},
    };
    use quent_events::{Event, EventData, engine, resource};
    use quent_time::bin::BinnedSpan;

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
            entities.custom_fsms.insert(
                fsm,
                fsm::Fsm {
                    id: fsm,
                    type_name: "test".to_string(),
                    instance_name: Some(format!("test-{i}")),
                    state_sequence: vec![
                        fsm::State {
                            name: "using".into(),
                            uses: vec![Use {
                                resource: resource_id,
                                capacities: vec![Attribute::u64("bytes", 250)],
                            }],
                            timestamp: i * 100,
                            attributes: vec![],
                            relations: vec![],
                        },
                        fsm::State {
                            name: "exit".into(),
                            uses: vec![],
                            timestamp: 1000 - i * 100,
                            attributes: vec![],
                            relations: vec![],
                        },
                    ],
                },
            );
        }

        let config = BinnedSpan::try_new(
            Span::try_new(0, 1000).unwrap(),
            NonZero::try_from(10).unwrap(),
        )
        .unwrap();

        let timeline =
            make_resource_timeline_bin_aggregated(&entities, resource_id, config).unwrap();

        // Config shouldn't be modified.
        assert_eq!(timeline.config, config);

        // We should have bin datapoints for the "bytes" capacity.
        assert!(timeline.capacity_values.contains_key("bytes"));

        let values = timeline.capacity_values.get("bytes").unwrap();

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
        entities.resources.insert(
            resource_id,
            Resource {
                id: resource_id,
                instance_name: Some("test".into()),
                type_name: "test".into(),
                scope: Some(EntityRef::Engine(engine_id)),
                capacities: HashMap::from([
                    (
                        "a".to_string(),
                        CapacityDecl::new_occupancy("a", Value::U64(0)),
                    ),
                    (
                        "b".to_string(),
                        CapacityDecl::new_occupancy("b", Value::U64(0)),
                    ),
                ]),
                state_sequence: vec![],
            },
        );

        // Spawn 2 FSMs using both capacities
        for i in 0..2 {
            let fsm = Uuid::now_v7();
            entities.custom_fsms.insert(
                fsm,
                fsm::Fsm {
                    id: fsm,
                    type_name: "test".to_string(),
                    instance_name: Some(format!("test-{i}")),
                    state_sequence: vec![
                        fsm::State {
                            name: "using".into(),
                            uses: vec![Use {
                                resource: resource_id,
                                capacities: vec![Attribute::u64("a", 250), Attribute::u64("b", 1)],
                            }],
                            timestamp: i * 250,
                            attributes: vec![],
                            relations: vec![],
                        },
                        fsm::State {
                            name: "exit".into(),
                            uses: vec![],
                            timestamp: 1000 - i * 250,
                            attributes: vec![],
                            relations: vec![],
                        },
                    ],
                },
            );
        }

        let config = BinnedSpan::try_new(
            Span::try_new(0, 1000).unwrap(),
            NonZero::try_from(10).unwrap(),
        )
        .unwrap();

        let timeline =
            make_resource_timeline_bin_aggregated(&entities, resource_id, config).unwrap();

        assert_eq!(timeline.config, config);
        assert!(timeline.capacity_values.contains_key("a"));
        assert!(timeline.capacity_values.contains_key("b"));

        let a = timeline.capacity_values.get("a").unwrap();
        let b = timeline.capacity_values.get("b").unwrap();

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
}
