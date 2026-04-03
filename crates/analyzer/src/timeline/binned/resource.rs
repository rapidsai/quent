// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Binned timelines for resource utilization.

use std::hash::Hash;

use quent_time::{SpanNanoSec, TimeNanoSec, bin::BinnedSpan};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use uuid::Uuid;

use crate::{
    AnalyzerResult,
    resource::{CapacityValue, ResourceTypeDecl, Usage},
    timeline::binned::{BinnedTimelineAggregator, KeyedAggregator},
};

/// Calculate a value to bin-aggregate depending on the [`CapacityType`].
fn convert_capacity(
    span: SpanNanoSec,
    capacity_value: &CapacityValue,
    resource_type: &ResourceTypeDecl,
) -> AnalyzerResult<f64> {
    let capacity_type = resource_type.try_capacity(capacity_value.name)?.kind;
    Ok(capacity_type.reinterpret_capacity_value(capacity_value.value.unwrap_or_default(), span))
}

#[derive(Clone, Debug)]
pub struct ResourceTimeline<'a> {
    pub config: BinnedSpan,
    pub data: HashMap<&'a str, Vec<f64>>,
    pub long_entities: Vec<Uuid>,
}

#[derive(Clone, Debug)]
pub struct ResourceTimelineByKey<'a, K> {
    pub config: BinnedSpan,
    pub data: HashMap<(K, &'a str), Vec<f64>>,
    pub long_entities: Vec<Uuid>,
}

pub struct ResourceTimelineBuilder<'a> {
    resource_type: &'a ResourceTypeDecl,
    aggregator: KeyedAggregator<&'a str>,
    long_entities: HashSet<Uuid>,
    long_entities_threshold: Option<TimeNanoSec>,
}

impl<'a> ResourceTimelineBuilder<'a> {
    pub fn try_new(
        resource_type: &'a ResourceTypeDecl,
        config: BinnedSpan,
        long_entities_threshold: Option<TimeNanoSec>,
    ) -> AnalyzerResult<Self> {
        // Construct the aggregator.
        let aggregator = KeyedAggregator::new(config);
        Ok(Self {
            resource_type,
            aggregator,
            long_entities: HashSet::default(),
            long_entities_threshold,
        })
    }

    pub fn try_push(&mut self, usage: &impl Usage<'a>) -> AnalyzerResult<()> {
        // TODO(johanpel): perf is fine for now but at some point we want to consider preventing all the hashmaps.
        for capacity in usage.capacities() {
            if capacity.value.is_some() {
                let value = convert_capacity(usage.span(), capacity, self.resource_type)?;
                self.aggregator
                    .try_push(usage.span(), (capacity.name, value))?
            }
        }

        if let Some(threshold) = self.long_entities_threshold
            && usage.span().duration() > threshold
            && usage.span().intersects(&self.aggregator.config.span)
        {
            self.long_entities.insert(usage.entity_id());
        }
        Ok(())
    }

    pub fn try_extend(
        &mut self,
        items: impl Iterator<Item = impl Usage<'a>>,
    ) -> AnalyzerResult<()> {
        for usage in items {
            self.try_push(&usage)?
        }
        Ok(())
    }

    pub fn build(self) -> ResourceTimeline<'a> {
        ResourceTimeline {
            config: self.aggregator.config,
            data: self.aggregator.finish(),
            long_entities: self.long_entities.into_iter().collect(),
        }
    }
}

pub struct ResourceTimelineByKeyBuilder<'a, K> {
    resource_type: &'a ResourceTypeDecl,
    aggregator: KeyedAggregator<(K, &'a str)>,
    long_entities: HashSet<Uuid>,
    long_entities_threshold: Option<TimeNanoSec>,
}

impl<'a, K> ResourceTimelineByKeyBuilder<'a, K>
where
    K: Eq + Hash + Clone,
{
    pub fn try_new(
        resource_type: &'a ResourceTypeDecl,
        config: BinnedSpan,
        long_entities_threshold: Option<TimeNanoSec>,
    ) -> AnalyzerResult<Self> {
        let aggregator = KeyedAggregator::new(config);

        Ok(Self {
            resource_type,
            aggregator,
            long_entities: HashSet::default(),
            long_entities_threshold,
        })
    }

    pub fn try_push(&mut self, key: K, usage: &impl Usage<'a>) -> AnalyzerResult<()> {
        for capacity in usage.capacities() {
            if capacity.value.is_some() {
                let value = convert_capacity(usage.span(), capacity, self.resource_type)?;
                self.aggregator
                    .try_push(usage.span(), ((key.clone(), capacity.name), value))?
            }
        }

        if let Some(threshold) = self.long_entities_threshold
            && usage.span().duration() > threshold
            && usage.span().intersects(&self.aggregator.config.span)
        {
            self.long_entities.insert(usage.entity_id());
        }
        Ok(())
    }

    pub fn try_extend(
        &mut self,
        items: impl Iterator<Item = (K, impl Usage<'a>)>,
    ) -> AnalyzerResult<()> {
        for (key, usage) in items {
            self.try_push(key, &usage)?;
        }
        Ok(())
    }

    pub fn build(self) -> ResourceTimelineByKey<'a, K> {
        ResourceTimelineByKey {
            config: self.aggregator.config,
            data: self.aggregator.finish(),
            long_entities: self.long_entities.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use crate::{
        fsm::{
            FsmUsages,
            collection::{FsmCollection, InMemoryFsms},
            runtime::{RtFsm, RtFsmStateUsage, RtFsmTransition},
        },
        resource::{
            CapacityDecl, Using,
            collection::{InMemoryResourcesBuilder, ResourceCollection},
            runtime::RtResource,
            tree::ResourceTreeNode,
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

        let mut fsms = InMemoryFsms::<RtFsm, RtFsmTransition>::new();

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
                        RtFsmTransition {
                            name: "using".into(),
                            usages: vec![RtFsmStateUsage::new(
                                resource_id,
                                [CapacityValue::new("bytes", 250)],
                            )],
                            timestamp: start,
                            attributes: vec![],
                        },
                        RtFsmTransition {
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

        let mut builder = ResourceTimelineBuilder::try_new(
            resources
                .resource_type(resources.resource(resource_id).unwrap().type_name())
                .unwrap(),
            config,
            None,
        )
        .unwrap();
        builder
            .try_extend(fsms.usages().filter(|u| u.resource_id() == resource_id))
            .unwrap();
        let timeline = builder.build();

        // Config shouldn't be modified.
        assert_eq!(timeline.config, config);

        // We should have bin datapoints for the "bytes" capacity.
        assert!(timeline.data.contains_key("bytes"));

        let values = timeline.data.get("bytes").unwrap();

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

        let mut fsms = InMemoryFsms::<RtFsm, RtFsmTransition>::new();

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
                        RtFsmTransition {
                            name: "using".into(),
                            usages: vec![RtFsmStateUsage::new(
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
                        RtFsmTransition {
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

        let group_resources = ResourceTreeNode::try_new(&resources, ROOT_RESOURCE_ID)
            .unwrap()
            .iter_leaf_refs(&resources)
            .filter_map(|maybe_resource| {
                maybe_resource
                    .ok()
                    .and_then(|r| (r.type_name() == "test").then_some(r.id()))
            })
            .collect::<HashSet<_>>();

        let mut builder = ResourceTimelineBuilder::try_new(
            resources.resource_type("test").unwrap(),
            BinnedSpan::try_new(
                SpanNanoSec::try_new(0, 1000).unwrap(),
                NonZero::try_from(10).unwrap(),
            )
            .unwrap(),
            None,
        )
        .unwrap();
        builder
            .try_extend(
                fsms.usages()
                    .filter(|u| group_resources.contains(&u.resource_id())),
            )
            .unwrap();
        let timeline = builder.build();

        let values = timeline.data.get("bytes").unwrap();

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

        let mut fsms = InMemoryFsms::<RtFsm, RtFsmTransition>::new();

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
            transitions: vec![],
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
                        RtFsmTransition {
                            name: "using".into(),
                            usages: vec![RtFsmStateUsage::new(
                                resource_id,
                                &[CapacityValue::new("a", 250), CapacityValue::new("b", 1)]
                                    as &[CapacityValue],
                            )],
                            timestamp: i * 250,
                            attributes: vec![],
                        },
                        RtFsmTransition {
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

        let mut builder = ResourceTimelineBuilder::try_new(
            resources.resource_type_of(resource_id).unwrap(),
            config,
            None,
        )
        .unwrap();
        builder
            .try_extend(fsms.usages().filter(|u| u.resource_id() == resource_id))
            .unwrap();
        let timeline = builder.build();

        assert_eq!(timeline.config, config);
        assert!(timeline.data.contains_key("a"));
        assert!(timeline.data.contains_key("b"));

        let a = timeline.data.get("a").unwrap();
        let b = timeline.data.get("b").unwrap();

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

        let mut fsms = InMemoryFsms::<RtFsm, RtFsmTransition>::new();

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
                        RtFsmTransition {
                            name: "state_a".into(),
                            usages: vec![RtFsmStateUsage::new(
                                resource_id,
                                [CapacityValue::new("bytes", 250)],
                            )],
                            timestamp: start,
                            attributes: vec![],
                        },
                        RtFsmTransition {
                            name: "state_b".into(),
                            usages: vec![RtFsmStateUsage::new(
                                resource_id,
                                [CapacityValue::new("bytes", 42)],
                            )],
                            timestamp: start + (end - start) / 2,
                            attributes: vec![],
                        },
                        RtFsmTransition {
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

        let mut builder = ResourceTimelineByKeyBuilder::try_new(
            resources
                .resource_type(resources.resource(resource_id).unwrap().type_name())
                .unwrap(),
            config,
            None,
        )
        .unwrap();

        for fsm in fsms.fsms() {
            for (state_name, usage) in fsm.usages_with_state_names() {
                if usage.resource_id() == resource_id {
                    builder.try_push(state_name, &usage).unwrap();
                }
            }
        }

        let timeline = builder.build();

        // Config shouldn't be modified.
        assert_eq!(timeline.config, config);

        // For each capacity, we should have values per state

        let state_a_bytes = timeline.data.get(&("state_a", "bytes")).unwrap();
        let state_b_bytes = timeline.data.get(&("state_b", "bytes")).unwrap();

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

        let mut fsms = InMemoryFsms::<RtFsm, RtFsmTransition>::new();

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
                        RtFsmTransition {
                            name: "state_a".into(),
                            usages: vec![RtFsmStateUsage::new(
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
                        RtFsmTransition {
                            name: "state_b".into(),
                            usages: vec![RtFsmStateUsage::new(
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
                        RtFsmTransition {
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

        let group_resources = ResourceTreeNode::try_new(&resources, ROOT_RESOURCE_ID)
            .unwrap()
            .iter_leaf_refs(&resources)
            .filter_map(|maybe_resource| {
                maybe_resource
                    .ok()
                    .and_then(|r| (r.type_name() == "test").then_some(r.id()))
            })
            .collect::<HashSet<_>>();

        let mut builder = ResourceTimelineByKeyBuilder::try_new(
            resources.resource_type("test").unwrap(),
            config,
            None,
        )
        .unwrap();
        for fsm in fsms.fsms() {
            for (state_name, usage) in fsm.usages_with_state_names() {
                if group_resources.contains(&usage.resource_id()) {
                    builder.try_push(state_name, &usage).unwrap();
                }
            }
        }
        let timeline = builder.build();

        let state_a_bytes = timeline.data.get(&("state_a", "bytes")).unwrap();
        let state_b_bytes = timeline.data.get(&("state_b", "bytes")).unwrap();

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

    /// Don't include long entities outside the window.
    #[test]
    fn test_long_entities_outside_window_excluded() {
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

        // Config window: [1000, 2000], threshold: 100 ns (all spans below exceed it)
        let config = BinnedSpan::try_new(
            SpanNanoSec::try_new(1000, 2000).unwrap(),
            NonZero::try_from(10).unwrap(),
        )
        .unwrap();
        let threshold = 100u64;

        let resource_type = resources
            .resource_type(resources.resource(resource_id).unwrap().type_name())
            .unwrap();

        let make_fsm = |start, end| {
            RtFsm::try_new(
                Uuid::now_v7(),
                "test",
                "test",
                [
                    RtFsmTransition {
                        name: "using".into(),
                        usages: vec![RtFsmStateUsage::new(
                            resource_id,
                            [CapacityValue::new("bytes", 1)],
                        )],
                        timestamp: start,
                        attributes: vec![],
                    },
                    RtFsmTransition {
                        name: "exit".into(),
                        usages: vec![],
                        timestamp: end,
                        attributes: vec![],
                    },
                ],
            )
            .unwrap()
        };

        let mut outside_fsms = InMemoryFsms::<RtFsm, RtFsmTransition>::new();
        outside_fsms.insert(make_fsm(0, 500));
        outside_fsms.insert(make_fsm(2500, 3000));

        let mut outside_builder =
            ResourceTimelineBuilder::try_new(resource_type, config, Some(threshold)).unwrap();
        outside_builder.try_extend(outside_fsms.usages()).unwrap();
        assert!(!outside_builder.build().long_entities.contains(&resource_id));

        let mut inside_fsms = InMemoryFsms::<RtFsm, RtFsmTransition>::new();
        inside_fsms.insert(make_fsm(500, 1500));
        inside_fsms.insert(make_fsm(1100, 1900));

        let mut inside_builder =
            ResourceTimelineBuilder::try_new(resource_type, config, Some(threshold)).unwrap();
        inside_builder.try_extend(inside_fsms.usages()).unwrap();
        assert!(inside_builder.build().long_entities.contains(&resource_id));
    }
}
