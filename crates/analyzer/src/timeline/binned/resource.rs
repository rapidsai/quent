//! Binned timelines for resource utilization.

use std::collections::{HashMap, HashSet};

use quent_time::{SpanNanoSec, bin::BinnedSpan};
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult,
    fsm::{Fsm, State, collection::FsmCollection},
    resource::{
        CapacityValue, ResourceTypeDecl, Using, collection::ResourceCollection,
        tree::ResourceTreeNode,
    },
    timeline::binned::{BinnedTimelineAggregator, NamedAggregator},
};

#[derive(Clone, Debug)]
pub struct ResourceTimelineBinned {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpan,
    /// Maps a resource capacity name to a vector where each element holds an
    /// aggregated value of a time bin.
    pub capacities_values: HashMap<String, Vec<f64>>,
}

#[derive(Clone, Debug)]
pub struct ResourceTimelineBinnedByState {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpan,
    /// Maps a resource capacity name to a map of a state name to a vector where
    /// each element holds an aggregated value of a time bin.
    pub capacities_states_values: HashMap<String, HashMap<String, Vec<f64>>>,
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

impl ResourceTimelineBinned {
    pub fn try_new_resource(
        resources: &impl ResourceCollection,
        users: &impl Using,
        resource_id: Uuid,
        config: BinnedSpan,
    ) -> AnalyzerResult<ResourceTimelineBinned> {
        let resource = resources.resource(resource_id)?;
        let resource_type = resources.resource_type(resource.type_name())?;

        let mut aggregator = NamedAggregator::new(config);

        for (span, capacity) in users.usages().flat_map(|(usage, span)| {
            usage
                .capacities
                .iter()
                .filter_map(move |cap| (usage.resource == resource_id).then_some((span, cap)))
        }) {
            if capacity.value.is_some() {
                let value = convert_capacity(span, capacity, resource_type)?;
                aggregator.try_push(span, (value, capacity.name.as_str()))?
            }
        }

        Ok(ResourceTimelineBinned {
            config,
            capacities_values: aggregator
                .try_finish()?
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        })
    }

    pub fn try_new_group(
        resources: &impl ResourceCollection,
        users: &impl Using,
        resource_group_id: Uuid,
        resource_type_name: &str,
        config: BinnedSpan,
    ) -> AnalyzerResult<ResourceTimelineBinned> {
        // Construct the tree with the provided resource group as root.
        let tree = ResourceTreeNode::try_new(resources, resource_group_id)?;
        // Look up the resource type
        let resource_type = resources.resource_type(resource_type_name)?;
        // Iterate over all leaf reasources, only keeping those of the requested type.
        let resources = tree
            .iter_leaves()
            .filter_map(|resource_id| {
                resources
                    .resource(resource_id)
                    .map(|resource| {
                        (resource.type_name() == resource_type_name).then_some(resource.id())
                    })
                    .transpose()
            })
            .collect::<AnalyzerResult<HashSet<_>>>()?;

        let mut aggregator = NamedAggregator::new(config);

        for (span, capacity) in users.usages().flat_map(|(usage, span)| {
            let resources = &resources;
            usage
                .capacities
                .iter()
                .filter_map(move |cap| resources.contains(&usage.resource).then_some((span, cap)))
        }) {
            if capacity.value.is_some() {
                let value = convert_capacity(span, capacity, resource_type)?;
                aggregator.try_push(span, (value, capacity.name.as_str()))?
            }
        }

        Ok(ResourceTimelineBinned {
            config,
            capacities_values: aggregator
                .try_finish()?
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        })
    }
}

impl ResourceTimelineBinnedByState {
    pub fn try_new_resource<T>(
        resources: &impl ResourceCollection,
        fsms: &impl FsmCollection<T>,
        resource_id: Uuid,
        fsm_type_name: &str,
        config: BinnedSpan,
    ) -> AnalyzerResult<ResourceTimelineBinnedByState>
    where
        T: Fsm,
        <T as Fsm>::StateType: Using,
    {
        if !fsms.contains_fsm_type(fsm_type_name) {
            return Err(AnalyzerError::InvalidArgument(format!(
                "fsm collection has no fsm type {fsm_type_name}"
            )));
        }

        let resource = resources.resource(resource_id)?;
        let resource_type = resources.resource_type(resource.type_name())?;

        let mut aggregators: HashMap<&str, NamedAggregator> = HashMap::new();

        for (state, capacity) in fsms
            .fsms()
            // Filter out any FSMs not of this type.
            .filter(|fsm| fsm.type_name() == fsm_type_name)
            // Flatten state usages
            .flat_map(|fsm| {
                fsm.states()
                    .flat_map(|state| state.usages().map(move |(usage, _)| (state, usage)))
                    // Filter out any usage not targeting the provided resource
                    .filter(|(_, usage)| usage.resource == resource_id)
            })
            // Flatten resource capacities
            .flat_map(|(state, usage)| {
                usage
                    .capacities
                    .iter()
                    .map(move |capacity| (state, capacity))
            })
        {
            let aggregator = aggregators
                .entry(capacity.name.as_str())
                .or_insert_with(|| NamedAggregator::new(config));
            if capacity.value.is_some() {
                let value = convert_capacity(state.span(), capacity, resource_type)?;
                aggregator.try_push(state.span(), (value, state.name()))?
            }
        }

        let capacities_states_values = aggregators
            .into_iter()
            .map(|(capacity, aggregator)| {
                aggregator.try_finish().map(|timeline| {
                    (
                        capacity.to_string(),
                        timeline
                            .into_iter()
                            .map(|(k, v)| (k.to_string(), v))
                            .collect(),
                    )
                })
            })
            .collect::<AnalyzerResult<_>>()?;

        Ok(ResourceTimelineBinnedByState {
            config,
            capacities_states_values,
        })
    }

    pub fn try_new_group<T>(
        resources: &impl ResourceCollection,
        fsms: &impl FsmCollection<T>,
        resource_group_id: Uuid,
        resource_type_name: &str,
        fsm_type_name: &str,
        config: BinnedSpan,
    ) -> AnalyzerResult<ResourceTimelineBinnedByState>
    where
        T: Fsm,
        <T as Fsm>::StateType: Using,
    {
        if !fsms.contains_fsm_type(fsm_type_name) {
            return Err(AnalyzerError::InvalidArgument(format!(
                "fsm collection has no fsm type {fsm_type_name}"
            )));
        }
        let tree = ResourceTreeNode::try_new(resources, resource_group_id)?;
        let resource_type = resources.resource_type(resource_type_name)?;
        let resources = tree
            .iter_leaves()
            .filter_map(|resource_id| {
                resources
                    .resource(resource_id)
                    .map(|resource| {
                        (resource.type_name() == resource_type_name).then_some(resource.id())
                    })
                    .transpose()
            })
            .collect::<AnalyzerResult<HashSet<_>>>()?;

        let mut aggregators: HashMap<&str, NamedAggregator> = HashMap::new();

        for (state, capacity) in fsms
            .fsms()
            // Filter out any FSMs not of this type.
            .filter(|fsm| fsm.type_name() == fsm_type_name)
            // Flatten state usages
            .flat_map(|fsm| {
                fsm.states()
                    .flat_map(|state| state.usages().map(move |(usage, _)| (state, usage)))
                    // Filter out any usage not targeting the provided resource
                    .filter(|(_, usage)| resources.contains(&usage.resource))
            })
            // Flatten resource capacities
            .flat_map(|(state, usage)| {
                usage
                    .capacities
                    .iter()
                    .map(move |capacity| (state, capacity))
            })
        {
            let aggregator = aggregators
                .entry(capacity.name.as_str())
                .or_insert_with(|| NamedAggregator::new(config));
            if capacity.value.is_some() {
                let value = convert_capacity(state.span(), capacity, resource_type)?;
                aggregator.try_push(state.span(), (value, state.name()))?
            }
        }

        let capacities_states_values = aggregators
            .into_iter()
            .map(|(capacity, aggregator)| {
                aggregator.try_finish().map(|timeline| {
                    (
                        capacity.to_string(),
                        timeline
                            .into_iter()
                            .map(|(k, v)| (k.to_string(), v))
                            .collect(),
                    )
                })
            })
            .collect::<AnalyzerResult<_>>()?;

        Ok(ResourceTimelineBinnedByState {
            config,
            capacities_states_values,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use crate::{
        fsm::{
            collection::InMemoryFsms,
            runtime::{RtFsm, RtTransition},
        },
        resource::{
            CapacityDecl, Usage, collection::InMemoryResourcesBuilder, runtime::RtResource,
        },
    };

    use super::*;

    use quent_events::{
        Event,
        resource::{self, GroupEvent, ResourceEvent},
    };
    use quent_time::{SpanNanoSec, bin::BinnedSpan};

    const ROOT_RESOURCE_ID: Uuid = Uuid::from_u64_pair(0, 1);

    fn root_resource_event() -> [Event<ResourceEvent>; 1] {
        [Event::new(
            ROOT_RESOURCE_ID,
            0,
            ResourceEvent::Group(GroupEvent {
                type_name: "test".into(),
                instance_name: "test".into(),
                parent_group_id: None,
            }),
        )]
    }

    fn memory_events(resource_id: Uuid) -> [Event<ResourceEvent>; 4] {
        [
            Event::new(
                resource_id,
                0,
                ResourceEvent::Memory(resource::memory::MemoryEvent::Init(
                    resource::memory::Init {
                        resource: resource::Resource {
                            instance_name: "test_inst".to_string(),
                            type_name: "test".to_string(),
                            parent_group_id: ROOT_RESOURCE_ID,
                        },
                    },
                )),
            ),
            Event::new(
                resource_id,
                0,
                ResourceEvent::Memory(resource::memory::MemoryEvent::Operating(
                    resource::memory::Operating {
                        capacity_bytes: 1000,
                    },
                )),
            ),
            Event::new(
                resource_id,
                1000,
                ResourceEvent::Memory(resource::memory::MemoryEvent::Finalizing(
                    resource::memory::Finalizing {
                        unreclaimed_bytes: 0,
                    },
                )),
            ),
            Event::new(
                resource_id,
                1000,
                ResourceEvent::Memory(resource::memory::MemoryEvent::Exit(
                    resource::memory::Exit {},
                )),
            ),
        ]
    }

    #[test]
    fn test_resource_timeline_aggregated() {
        let resource_id = Uuid::now_v7();

        let mut resources = InMemoryResourcesBuilder::default();
        resources
            .try_extend(
                root_resource_event()
                    .into_iter()
                    .chain(memory_events(resource_id)),
            )
            .unwrap();
        let resources = resources.try_build().unwrap();

        let mut fsms = InMemoryFsms::<RtFsm>::new();

        // Produce triangle-ish memory utilization using 4 FSMs
        for i in 0..4 {
            let fsm = Uuid::now_v7();
            let start = i * 100;
            let end = 1000 - i * 100;
            fsms.insert(
                RtFsm::try_new(
                    fsm,
                    "test",
                    "test",
                    [
                        RtTransition {
                            name: "using".into(),
                            usages: vec![Usage::new(
                                resource_id,
                                [CapacityValue::new("bytes", 250)],
                            )],
                            timestamp: start,
                            attributes: vec![],
                        },
                        RtTransition {
                            name: "exit".into(),
                            usages: vec![],
                            timestamp: end,
                            attributes: vec![],
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

        let timeline =
            ResourceTimelineBinned::try_new_resource(&resources, &fsms, resource_id, config)
                .unwrap();

        // Config shouldn't be modified.
        assert_eq!(timeline.config, config);

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
        let resource_a_id = Uuid::now_v7();
        let resource_b_id = Uuid::now_v7();

        let mut resources = InMemoryResourcesBuilder::default();
        resources
            .try_extend(
                root_resource_event()
                    .into_iter()
                    .chain(memory_events(resource_a_id))
                    .chain(memory_events(resource_b_id)),
            )
            .unwrap();
        let resources = resources.try_build().unwrap();

        let mut fsms = InMemoryFsms::<RtFsm>::new();

        // Produce triangle-ish memory utilization using 4 FSMs
        for i in 0..4 {
            let fsm = Uuid::now_v7();
            let start = i * 100;
            let end = 1000 - i * 100;
            fsms.insert(
                RtFsm::try_new(
                    fsm,
                    "test",
                    "test",
                    [
                        RtTransition {
                            name: "using".into(),
                            usages: vec![Usage::new(
                                if i % 2 == 0 {
                                    resource_a_id
                                } else {
                                    resource_b_id
                                },
                                [CapacityValue::new("bytes", 250)],
                            )],
                            timestamp: start,
                            attributes: vec![],
                        },
                        RtTransition {
                            name: "exit".into(),
                            usages: vec![],
                            timestamp: end,
                            attributes: vec![],
                        },
                    ],
                )
                .unwrap(),
            );
        }

        let timeline = ResourceTimelineBinned::try_new_group(
            &resources,
            &fsms,
            ROOT_RESOURCE_ID,
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
        let resource_id = Uuid::now_v7();

        let mut resources = InMemoryResourcesBuilder::default();
        resources
            .try_extend(root_resource_event().into_iter())
            .unwrap();
        let mut resources = resources.try_build().unwrap();

        let mut fsms = InMemoryFsms::<RtFsm>::new();

        // Add a resource with 2 capacities.
        resources.insert_type(ResourceTypeDecl::new(
            "test",
            &[
                CapacityDecl::new_occupancy("a"),
                CapacityDecl::new_occupancy("b"),
            ][..],
        ));
        resources.insert_resource(RtResource {
            id: resource_id,
            instance_name: "test".into(),
            type_name: "test".into(),
            parent_group_id: ROOT_RESOURCE_ID,
            sequence: vec![],
        });

        // Spawn 2 FSMs using both capacities
        for i in 0..2 {
            let fsm = Uuid::now_v7();
            fsms.insert(
                RtFsm::try_new(
                    fsm,
                    "test",
                    "test",
                    [
                        RtTransition {
                            name: "using".into(),
                            usages: vec![Usage::new(
                                resource_id,
                                &[CapacityValue::new("a", 250), CapacityValue::new("b", 1)]
                                    as &[CapacityValue],
                            )],
                            timestamp: i * 250,
                            attributes: vec![],
                        },
                        RtTransition {
                            name: "exit".into(),
                            usages: vec![],
                            timestamp: 1000 - i * 250,
                            attributes: vec![],
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

        let timeline =
            ResourceTimelineBinned::try_new_resource(&resources, &fsms, resource_id, config)
                .unwrap();

        assert_eq!(timeline.config, config);
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
        let resource_id = Uuid::now_v7();

        let mut resources = InMemoryResourcesBuilder::default();
        resources
            .try_extend(
                root_resource_event()
                    .into_iter()
                    .chain(memory_events(resource_id)),
            )
            .unwrap();
        let resources = resources.try_build().unwrap();

        let mut fsms = InMemoryFsms::<RtFsm>::new();

        // Produce triangle-ish memory utilization using 4 FSMs with 2 usage states
        for i in 0..4 {
            let fsm = Uuid::now_v7();
            let start = 100 * i;
            let end = 1000 - i * 100;
            fsms.insert(
                RtFsm::try_new(
                    fsm,
                    "test",
                    "test",
                    [
                        RtTransition {
                            name: "state_a".into(),
                            usages: vec![Usage::new(
                                resource_id,
                                [CapacityValue::new("bytes", 250)],
                            )],
                            timestamp: start,
                            attributes: vec![],
                        },
                        RtTransition {
                            name: "state_b".into(),
                            usages: vec![Usage::new(
                                resource_id,
                                [CapacityValue::new("bytes", 42)],
                            )],
                            timestamp: start + (end - start) / 2,
                            attributes: vec![],
                        },
                        RtTransition {
                            name: "exit".into(),
                            usages: vec![],
                            timestamp: end,
                            attributes: vec![],
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

        let timeline = ResourceTimelineBinnedByState::try_new_resource(
            &resources,
            &fsms,
            resource_id,
            "test",
            config,
        )
        .unwrap();

        // Config shouldn't be modified.
        assert_eq!(timeline.config, config);

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
        let resource_a_id = Uuid::now_v7();
        let resource_b_id = Uuid::now_v7();

        let mut resources = InMemoryResourcesBuilder::default();
        resources
            .try_extend(
                root_resource_event()
                    .into_iter()
                    .chain(memory_events(resource_a_id))
                    .chain(memory_events(resource_b_id)),
            )
            .unwrap();
        let resources = resources.try_build().unwrap();

        let mut fsms = InMemoryFsms::<RtFsm>::new();

        // Produce the same utilization as previous test but spread it out across two leaf resources.
        for i in 0..4 {
            let fsm = Uuid::now_v7();
            let start = 100 * i;
            let end = 1000 - i * 100;
            fsms.insert(
                RtFsm::try_new(
                    fsm,
                    "test",
                    "test",
                    [
                        RtTransition {
                            name: "state_a".into(),
                            usages: vec![Usage::new(
                                if i % 2 == 0 {
                                    resource_a_id
                                } else {
                                    resource_b_id
                                },
                                [CapacityValue::new("bytes", 250)],
                            )],
                            timestamp: start,
                            attributes: vec![],
                        },
                        RtTransition {
                            name: "state_b".into(),
                            usages: vec![Usage::new(
                                if i % 2 == 0 {
                                    resource_a_id
                                } else {
                                    resource_b_id
                                },
                                [CapacityValue::new("bytes", 42)],
                            )],
                            timestamp: start + (end - start) / 2,
                            attributes: vec![],
                        },
                        RtTransition {
                            name: "exit".into(),
                            usages: vec![],
                            timestamp: end,
                            attributes: vec![],
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
        let timeline = ResourceTimelineBinnedByState::try_new_group(
            &resources,
            &fsms,
            ROOT_RESOURCE_ID,
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
