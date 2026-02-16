//! Binned timelines for resource utilization.

use std::{collections::HashSet, hash::Hash};

use quent_time::{SpanNanoSec, bin::BinnedSpan, span::SpanUnixNanoSec};
use rustc_hash::FxHashMap as HashMap;
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
}

#[derive(Clone, Debug)]
pub struct ResourceTimelineByKey<'a, K> {
    pub config: BinnedSpan,
    pub data: HashMap<(K, &'a str), Vec<f64>>,
}

pub trait ResourceIdFilter {
    fn accepts(&self, id: Uuid) -> bool;
}

impl ResourceIdFilter for Uuid {
    fn accepts(&self, id: Uuid) -> bool {
        *self == id
    }
}

impl ResourceIdFilter for HashSet<Uuid> {
    fn accepts(&self, id: Uuid) -> bool {
        self.contains(&id)
    }
}

pub struct ResourceTimelineBuilder<'a, F>
where
    F: ResourceIdFilter,
{
    resource_type: &'a ResourceTypeDecl,
    id_filter: F,
    aggregator: KeyedAggregator<&'a str>,
}

impl<'a, F> ResourceTimelineBuilder<'a, F>
where
    F: ResourceIdFilter,
{
    pub fn try_new(
        resource_type: &'a ResourceTypeDecl,
        id_filter: F,
        config: BinnedSpan,
    ) -> AnalyzerResult<Self> {
        // Construct the aggregator.
        let aggregator = KeyedAggregator::new(config);
        Ok(Self {
            id_filter,
            resource_type,
            aggregator,
        })
    }

    pub fn id_filter(&self) -> &F {
        &self.id_filter
    }

    pub fn try_push(&mut self, usage: &'a Usage, span: SpanUnixNanoSec) -> AnalyzerResult<()> {
        if self.id_filter.accepts(usage.resource) {
            self.try_push_prefiltered(usage, span)?;
        }
        Ok(())
    }

    /// Push a usage without checking the id filter.
    /// Caller must guarantee that `usage.resource` is accepted by this builder.
    pub fn try_push_prefiltered(
        &mut self,
        usage: &'a Usage,
        span: SpanUnixNanoSec,
    ) -> AnalyzerResult<()> {
        // TODO(johanpel): perf is fine for now but at some point we want to consider preventing all the hashmaps.
        for capacity in usage.capacities.iter() {
            if capacity.value.is_some() {
                let value = convert_capacity(span, capacity, self.resource_type)?;
                self.aggregator.try_push(span, (capacity.name, value))?
            }
        }
        Ok(())
    }

    pub fn try_extend(
        &mut self,
        items: impl Iterator<Item = (&'a Usage, SpanUnixNanoSec)>,
    ) -> AnalyzerResult<()> {
        for (usage, span) in items {
            self.try_push(usage, span)?
        }
        Ok(())
    }

    pub fn build(self) -> ResourceTimeline<'a> {
        ResourceTimeline {
            config: self.aggregator.config,
            data: self.aggregator.finish(),
        }
    }
}

pub struct ResourceTimelineByKeyBuilder<'a, F, K>
where
    F: ResourceIdFilter,
{
    resource_type: &'a ResourceTypeDecl,
    id_filter: F,
    aggregator: KeyedAggregator<(K, &'a str)>,
}

impl<'a, F, K> ResourceTimelineByKeyBuilder<'a, F, K>
where
    F: ResourceIdFilter,
    K: Eq + Hash + Copy,
{
    pub fn try_new(
        resource_type: &'a ResourceTypeDecl,
        id_filter: F,
        config: BinnedSpan,
    ) -> AnalyzerResult<Self> {
        let aggregator = KeyedAggregator::new(config);

        Ok(Self {
            resource_type,
            id_filter,
            aggregator,
        })
    }

    pub fn id_filter(&self) -> &F {
        &self.id_filter
    }

    pub fn try_push(
        &mut self,
        key: K,
        usage: &'a Usage,
        span: SpanUnixNanoSec,
    ) -> AnalyzerResult<()> {
        if self.id_filter.accepts(usage.resource) {
            self.try_push_prefiltered(key, usage, span)?;
        }
        Ok(())
    }

    /// Push a usage without checking the id filter.
    /// Caller must guarantee that `usage.resource` is accepted by this builder.
    pub fn try_push_prefiltered(
        &mut self,
        key: K,
        usage: &'a Usage,
        span: SpanUnixNanoSec,
    ) -> AnalyzerResult<()> {
        for capacity in usage.capacities.iter() {
            if capacity.value.is_some() {
                let value = convert_capacity(span, capacity, self.resource_type)?;
                self.aggregator
                    .try_push(span, ((key, capacity.name), value))?
            }
        }
        Ok(())
    }

    pub fn try_extend(
        &mut self,
        items: impl Iterator<Item = (K, &'a Usage, SpanUnixNanoSec)>,
    ) -> AnalyzerResult<()> {
        for (key, usage, span) in items {
            self.try_push(key, usage, span)?;
        }
        Ok(())
    }

    pub fn build(self) -> ResourceTimelineByKey<'a, K> {
        ResourceTimelineByKey {
            config: self.aggregator.config,
            data: self.aggregator.finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use crate::{
        fsm::{
            Fsm, State,
            collection::{FsmCollection, InMemoryFsms},
            runtime::{RtFsm, RtTransition},
        },
        resource::{
            CapacityDecl, Usage, Using,
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

        let mut builder = ResourceTimelineBuilder::try_new(
            resources
                .resource_type(resources.resource(resource_id).unwrap().type_name())
                .unwrap(),
            resource_id,
            config,
        )
        .unwrap();
        builder.try_extend(fsms.usages()).unwrap();
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
            group_resources,
            BinnedSpan::try_new(
                SpanNanoSec::try_new(0, 1000).unwrap(),
                NonZero::try_from(10).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        builder.try_extend(fsms.usages()).unwrap();
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

        let mut builder = ResourceTimelineBuilder::try_new(
            resources.resource_type_of(resource_id).unwrap(),
            resource_id,
            config,
        )
        .unwrap();
        builder.try_extend(fsms.usages()).unwrap();
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

        let mut builder = ResourceTimelineByKeyBuilder::try_new(
            resources
                .resource_type(resources.resource(resource_id).unwrap().type_name())
                .unwrap(),
            resource_id,
            config,
        )
        .unwrap();
        builder
            .try_extend(fsms.fsms().flat_map(|fsm| {
                fsm.states().flat_map(|state| {
                    state
                        .usages()
                        .map(|(usage, span)| (state.name.as_str(), usage, span))
                })
            }))
            .unwrap();
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
            group_resources,
            config,
        )
        .unwrap();
        builder
            .try_extend(fsms.fsms().flat_map(|fsm| {
                fsm.states()
                    .flat_map(|s| s.usages().map(|u| (s.name(), u.0, u.1)))
            }))
            .unwrap();
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
}
