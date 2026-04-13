// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::Event;
pub use quent_query_engine_analyzer::QueryEngineModel;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use quent_query_engine_ui::{QueryBundle, QueryEntities};
use quent_ui::{
    FiniteStateMachine, ResourceGroupNode, ResourceTree, convert_resource_tree,
    quantity::QuantitySpec,
    timeline::{
        request::{BulkTimelineRequest, EntityFilter, SingleTimelineRequest, TimelineRequest},
        response::{
            BulkTimelinesResponse, BulkTimelinesResponseEntry,
            ResourceTimeline as UiResourceTimeline, ResourceTimelineBinned,
            ResourceTimelineBinnedByState, SingleTimelineResponse,
        },
    },
};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::collections::HashMap as StdHashMap;
use tracing::debug;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model, Span,
    fsm::{FsmTypeDeclaration, FsmUsages},
    resource::{
        ResourceTypeDecl, Usage, Using, collection::ResourceCollection, tree::ResourceTreeNode,
    },
    timeline::binned::resource::{
        ResourceTimeline, ResourceTimelineBuilder, ResourceTimelineByKey,
        ResourceTimelineByKeyBuilder,
    },
};
use quent_simulator_instrumentation::SimulatorEvent;
use quent_simulator_ui::{EntityRef, QueryFilter, TaskFilter};
use quent_time::{SpanNanoSec, TimeNanoSec, TimeUnixNanoSec, to_nanosecs, to_secs};
use uuid::Uuid;

use crate::{
    model::{SimulatorModel, SimulatorModelBuilder},
    task::{Task, TaskExt},
};

pub mod model;
pub mod task;
pub mod view;

pub struct SimulatorUiAnalyzer {
    pub model: SimulatorModel,
}

impl UiAnalyzer for SimulatorUiAnalyzer {
    type Event = SimulatorEvent;
    type EntityRef = EntityRef;
    type TimelineGlobalParams = QueryFilter;
    type TimelineParams = TaskFilter;

    fn extract_engine(
        engine_id: Uuid,
        events: impl Iterator<Item = Event<SimulatorEvent>>,
    ) -> AnalyzerResult<quent_query_engine_ui::Engine> {
        use quent_query_engine_model::engine::EngineEvent;
        for event in events {
            if let SimulatorEvent::Engine(EngineEvent::Init(init)) = event.data {
                return Ok(quent_query_engine_ui::Engine {
                    id: engine_id,
                    start_time_unix_ns: Some(event.timestamp),
                    duration_s: None,
                    instance_name: init.instance_name,
                    implementation: Some(
                        quent_query_engine_ui::EngineImplementationAttributes::from(
                            &init.implementation,
                        ),
                    ),
                });
            }
        }
        Ok(quent_query_engine_ui::Engine::new(engine_id))
    }

    fn try_new(
        engine_id: Uuid,
        events: impl Iterator<Item = Event<SimulatorEvent>>,
    ) -> AnalyzerResult<Self> {
        let mut builder = SimulatorModelBuilder::try_new(engine_id)?;
        {
            let _span = tracing::info_span!("ingest").entered();
            for event in events {
                builder.try_push(event)?;
            }
        }
        let model = {
            let _span = tracing::info_span!("build").entered();
            builder.try_build()?
        };

        let qe = &model.query_engine;
        tracing::info!(
            workers = qe.workers.len(),
            query_groups = qe.query_groups.len(),
            queries = qe.queries.len(),
            plans = qe.plans.len(),
            operators = qe.operators.len(),
            ports = qe.ports.len(),
            resources = model.arbitrary_resources.resources.len(),
            resource_groups = model.arbitrary_resources.resource_groups.len(),
            resource_types = model.arbitrary_resources.resource_types.len(),
            resource_group_types = model.resource_group_types.len(),
            tasks = model.tasks.len(),
        );

        Ok(Self { model })
    }

    fn query_bundle(&self, query_id: Uuid) -> AnalyzerResult<QueryBundle<EntityRef>> {
        debug!("constructing view");
        // TODO(johanpel): A query view could be cached in an analyzer so
        // subsequent calls into the analyzer for that query could benefit from
        // it.
        let view = self.model.query_view(query_id)?;
        let query = self.model.query(query_id)?;
        let start_time_unix_ns = view.query_epoch(query_id)?;
        let duration_s = to_secs(query.span()?.duration());
        let epoch = view.query_epoch(query_id)?;

        debug!("converting query engine model entities");
        let engine = view.engine()?.to_ui()?;
        let query_group_id = query.query_group_id().ok_or_else(|| {
            quent_analyzer::AnalyzerError::IncompleteEntity(format!(
                "query {} has no query_group_id",
                query_id
            ))
        })?;
        let query_group = view.query_group(query_group_id)?.to_ui();
        let query = query.to_ui()?;
        let workers = view.workers().map(|w| (w.id(), w.to_ui(epoch))).collect();
        let plans = view.plans().map(|p| (p.id(), p.to_ui())).collect();
        let operators = view.operators().map(|o| (o.id(), o.to_ui(epoch))).collect();
        let ports = view.ports().map(|p| (p.id(), p.to_ui(epoch))).collect();
        let unique_operator_names = view
            .operators()
            .filter_map(|v| v.operator_type_name().map(|s| s.to_owned()))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        debug!("converting simulator model entities");
        let resources = self
            .model
            .arbitrary_resources
            .resources()
            .map(|res| (res.id(), res.into()))
            .collect();
        let resource_types = self
            .model
            .arbitrary_resources
            .resource_types
            .iter()
            .map(|(k, v)| (k.clone(), v.into()))
            .collect();
        let resource_groups = self
            .model
            .arbitrary_resources
            .resource_groups()
            .map(|res| (res.id(), res.into()))
            .collect();
        let resource_group_types = self
            .model
            .resource_group_types
            .iter()
            .map(|(k, v)| (k.clone(), v.into()))
            .collect();

        let task_decl = Task::fsm_type_declaration();
        let fsm_types = [(task_decl.name.clone(), task_decl)].into_iter().collect();

        let entities = QueryEntities {
            engine,
            query_group,
            query,
            workers,
            plans,
            operators,
            ports,
            resource_types,
            resources,
            resource_groups,
            resource_group_types,
            fsm_types,
        };

        debug!("deriving plan tree");
        let plan_tree = view.plan_tree(query_id)?.to_ui();

        debug!("deriving resource tree");
        let engine = view.engine()?;
        let resource_tree =
            convert_resource_tree(view.resource_tree()?, &view)?.unwrap_or_else(|| {
                ResourceTree::ResourceGroup(ResourceGroupNode {
                    id: EntityRef::Engine(engine.id()),
                    children: vec![],
                })
            });

        Ok(QueryBundle {
            query_id,
            entities,
            plan_tree,
            resource_tree,
            unique_operator_names,
            quantity_specs: [
                ("bytes".into(), QuantitySpec::bytes()),
                ("unit".into(), QuantitySpec::unit()),
            ]
            .into(),
            start_time_unix_ns,
            duration_s,
        })
    }

    fn query_engine_model(&self) -> &impl QueryEngineModel {
        &self.model
    }

    // TODO(johanpel): consider re-using the bulk request API with a single entry for requests like this.
    fn single_resource_timeline(
        &self,
        request: SingleTimelineRequest<Self::TimelineGlobalParams, Self::TimelineParams>,
    ) -> AnalyzerResult<SingleTimelineResponse> {
        // TODO(johanpel): we may want to sanity-check whether the requested
        // resource/group is actually in the resource tree for a given query.

        // Calculate this ASAP to help fail quickly.
        let epoch = self
            .query_engine_model()
            .query_epoch(request.app_params.query_id)?;
        let config = request.entry.config().clone().try_into_binned_span(epoch)?;
        let config_secs = config.try_to_secs_relative(epoch)?;

        match request.entry {
            TimelineRequest::Resource(req) => {
                let resource_type = self.model.resource_type_of(req.resource_id)?;
                let long_entities_threshold = req.long_entities_threshold_s.map(to_nanosecs);
                let task_filter = req.application;

                if req.entity_filter.entity_type_name.is_some() {
                    let mut builder = ResourceTimelineByKeyBuilder::try_new(
                        resource_type,
                        config,
                        long_entities_threshold,
                    )?;
                    // This application only has Task FSM
                    self.populate_keyed_builder(
                        &mut builder,
                        self.entities_filtered(req.entity_filter, task_filter, config.span)?
                            .filter(|task| {
                                task.usages()
                                    .any(|usage| usage.resource_id() == req.resource_id)
                            }),
                        |id| id == req.resource_id,
                    )?;
                    Ok(SingleTimelineResponse {
                        config: config_secs,
                        data: self.timeline_to_ui_keyed(builder.build(), epoch)?,
                    })
                } else {
                    let mut builder = ResourceTimelineBuilder::try_new(
                        resource_type,
                        config,
                        long_entities_threshold,
                    )?;

                    builder.try_extend(
                        self.entities_filtered(req.entity_filter, task_filter, config.span)?
                            .flat_map(|task| task.usages())
                            .filter(|usage| usage.resource_id() == req.resource_id),
                    )?;
                    Ok(SingleTimelineResponse {
                        config: config_secs,
                        data: self.timeline_to_ui(builder.build(), epoch)?,
                    })
                }
            }
            TimelineRequest::ResourceGroup(req) => {
                let resource_type = self.model.resource_type(&req.resource_type_name)?;
                let long_entities_threshold = req.long_entities_threshold_s.map(to_nanosecs);

                // Build the resource tree for this group
                let tree = ResourceTreeNode::try_new(&self.model, req.resource_group_id)?;
                // Collect all leaf resource IDs of the requested type in the tree
                let resource_ids: HashSet<Uuid> = tree
                    .iter_leaf_ids()
                    .filter(|&id| {
                        self.model
                            .resource(id)
                            .ok()
                            .map(|r| r.type_name() == resource_type.name)
                            .unwrap_or(false)
                    })
                    .collect();

                if req.entity_filter.entity_type_name.is_some() {
                    let mut builder = ResourceTimelineByKeyBuilder::try_new(
                        resource_type,
                        config,
                        long_entities_threshold,
                    )?;
                    self.populate_keyed_builder(
                        &mut builder,
                        self.entities_filtered(req.entity_filter, req.app_params, config.span)?
                            .filter(|task| {
                                task.usages()
                                    .any(|usage| resource_ids.contains(&usage.resource_id()))
                            }),
                        |id| resource_ids.contains(&id),
                    )?;
                    Ok(SingleTimelineResponse {
                        config: config_secs,
                        data: self.timeline_to_ui_keyed(builder.build(), epoch)?,
                    })
                } else {
                    let mut builder = ResourceTimelineBuilder::try_new(
                        resource_type,
                        config,
                        long_entities_threshold,
                    )?;
                    builder.try_extend(
                        self.entities_filtered(req.entity_filter, req.app_params, config.span)?
                            .flat_map(|task| task.usages())
                            .filter(|usage| resource_ids.contains(&usage.resource_id())),
                    )?;
                    Ok(SingleTimelineResponse {
                        config: config_secs,
                        data: self.timeline_to_ui(builder.build(), epoch)?,
                    })
                }
            }
        }
    }

    fn bulk_resource_timeline(
        &self,
        request: BulkTimelineRequest<Self::TimelineGlobalParams, Self::TimelineParams>,
    ) -> AnalyzerResult<BulkTimelinesResponse> {
        // Calculate this ASAP to help fail quickly.
        let epoch = self
            .query_engine_model()
            .query_epoch(request.app_params.query_id)?;

        // Construct a query view.
        let view = self.model.query_view(request.app_params.query_id)?;
        // Prepare resource tree, we'll re-use this as it is potentially
        // expensive to build for every entry.
        let resource_tree = view.resource_tree()?;

        // Prepare builders, resource id filters, and operator filters, one for
        // each bulk entry. After populating this, we'll build a reverse index,
        // that maps a resource_id to a list of indices in these vecs, for which
        // that resource's usages are relevant.
        let mut plain_builders: Vec<(String, ResourceTimelineBuilder, HashSet<Uuid>, TaskFilter)> =
            Vec::new();

        // Prepare them also for keyed builders (building by state).
        let mut per_state_builders: Vec<(
            String,
            ResourceTimelineByKeyBuilder<&str>,
            HashSet<Uuid>,
            TaskFilter,
        )> = Vec::new();

        for (entry_id, entry) in request.entries {
            let entry_config = entry.config().clone().try_into_binned_span(epoch)?;
            let BulkEntryPrep {
                resource_type,
                resource_id_filter,
                entity_filter,
                task_filter,
                long_entities_threshold,
            } = self.try_prepare_bulk_entry(entry, &resource_tree)?;
            if entity_filter.entity_type_name.is_some() {
                per_state_builders.push((
                    entry_id,
                    ResourceTimelineByKeyBuilder::try_new(
                        resource_type,
                        entry_config,
                        long_entities_threshold,
                    )?,
                    resource_id_filter,
                    task_filter,
                ));
            } else {
                plain_builders.push((
                    entry_id,
                    ResourceTimelineBuilder::try_new(
                        resource_type,
                        entry_config,
                        long_entities_threshold,
                    )?,
                    resource_id_filter,
                    task_filter,
                ));
            }
        }

        // Build reverse index so given the id of an entry in the request, we
        // can quickly look up all builders associated with the entry into which
        // we can push a usage.
        //
        // This is more efficient than going over all usages for each builder,
        // since the number of usages is typically going to be MUCH larger than
        // the number of builders.
        let plain_index: HashMap<Uuid, Vec<usize>> = plain_builders
            .iter()
            .enumerate()
            .flat_map(|(builders_index, builder)| {
                builder
                    .2
                    .iter()
                    .map(move |&resource_id| (resource_id, builders_index))
            })
            .fold(
                HashMap::default(),
                |mut acc, (resource_id, builders_index)| {
                    acc.entry(resource_id).or_default().push(builders_index);
                    acc
                },
            );
        let per_state_index: HashMap<Uuid, Vec<usize>> = per_state_builders
            .iter()
            .enumerate()
            .flat_map(|(builders_index, builder)| {
                builder
                    .2
                    .iter()
                    .map(move |&resource_id| (resource_id, builders_index))
            })
            .fold(
                HashMap::default(),
                |mut acc, (resource_id, builders_index)| {
                    acc.entry(resource_id).or_default().push(builders_index);
                    acc
                },
            );

        // Iterate over all usages once and push any usages of resources in our
        // lookup table to their respective builders. For now we only have
        // tasks.
        for task in self.model.tasks.values() {
            let task_operator_id = task.operator_id();
            for usage in task.usages() {
                let resource_id = usage.resource_id();
                if let Some(builder_indices) = plain_index.get(&resource_id) {
                    for &builder_idx in builder_indices {
                        let builder = &mut plain_builders[builder_idx];
                        if builder
                            .3
                            .operator_id
                            .is_none_or(|op| task_operator_id == Some(op))
                        {
                            plain_builders[builder_idx].1.try_push(&usage)?;
                        }
                    }
                }
            }

            for (state_name, usage) in task.usages_with_state_names() {
                let resource_id = usage.resource_id();
                if let Some(builder_indices) = per_state_index.get(&resource_id) {
                    for &builder_idx in builder_indices {
                        let builder = &mut per_state_builders[builder_idx];
                        if builder
                            .3
                            .operator_id
                            .is_none_or(|op| task_operator_id == Some(op))
                        {
                            per_state_builders[builder_idx]
                                .1
                                .try_push(state_name, &usage)?;
                        }
                    }
                }
            }
        }

        // Collect results for all requests.
        let mut entries = std::collections::HashMap::default();
        for (entry_id, builder, _, _) in plain_builders {
            let built = builder.build();
            let config = built.config.try_to_secs_relative(epoch)?;
            entries.insert(
                entry_id,
                BulkTimelinesResponseEntry::Ok {
                    message: String::new(),
                    config,
                    data: self.timeline_to_ui(built, epoch)?,
                },
            );
        }
        for (key, builder, _, _) in per_state_builders {
            let built = builder.build();
            let config = built.config.try_to_secs_relative(epoch)?;
            entries.insert(
                key,
                BulkTimelinesResponseEntry::Ok {
                    message: String::new(),
                    config,
                    data: self.timeline_to_ui_keyed(built, epoch)?,
                },
            );
        }

        Ok(BulkTimelinesResponse { entries })
    }
}

impl SimulatorUiAnalyzer {
    /// Return an iterator over all tasks, filtered by time window and operator id.
    fn entities_filtered(
        &self,
        entity_filter: EntityFilter,
        task_filter: TaskFilter,
        time_window: SpanNanoSec,
    ) -> AnalyzerResult<Box<dyn Iterator<Item = &Task> + '_>> {
        if let Some(entity_type_name) = entity_filter.entity_type_name {
            match entity_type_name.as_str() {
                "task" => Ok(Box::new(self.model.tasks.values().filter(move |task| {
                    task_filter
                        .operator_id
                        .is_none_or(|op| task.operator_id() == Some(op))
                        && task.span().is_ok_and(|s| s.intersects(&time_window))
                }))),
                _ => Err(AnalyzerError::InvalidArgument(format!(
                    "{} is not a known entity type in this model",
                    entity_type_name
                ))),
            }
        } else {
            Ok(Box::new(self.model.tasks.values().filter(move |task| {
                task_filter
                    .operator_id
                    .is_none_or(|op| task.operator_id() == Some(op))
                    && task.span().is_ok_and(|s| s.intersects(&time_window))
            })))
        }
    }

    /// Given a TimelineRequest figure out what are:
    /// - The resource_type
    /// - For groups, the set of resources to aggregate for.
    /// - Whether this is a request to split out usage per state.
    /// - What operator ID filter to apply.
    /// - What the threshold is for long entities.
    fn try_prepare_bulk_entry<'a>(
        &'a self,
        request: TimelineRequest<TaskFilter>,
        tree: &ResourceTreeNode,
    ) -> AnalyzerResult<BulkEntryPrep<'a>> {
        Ok(match request {
            TimelineRequest::Resource(r) => BulkEntryPrep {
                resource_type: self.model.resource_type_of(r.resource_id)?,
                resource_id_filter: [r.resource_id].into_iter().collect(),
                entity_filter: r.entity_filter,
                task_filter: r.application,
                long_entities_threshold: r.long_entities_threshold_s.map(to_nanosecs),
            },
            TimelineRequest::ResourceGroup(rg) => {
                let resource_type = self.model.resource_type(&rg.resource_type_name)?;
                let subtree = tree
                    .find(rg.resource_group_id)
                    .ok_or(AnalyzerError::InvalidId(rg.resource_group_id))?;
                let resource_ids: HashSet<Uuid> = subtree
                    .iter_leaf_ids()
                    .filter(|&id| {
                        self.model
                            .resource(id)
                            .ok()
                            .is_some_and(|r| r.type_name() == rg.resource_type_name)
                    })
                    .collect();
                BulkEntryPrep {
                    resource_type,
                    resource_id_filter: resource_ids,
                    entity_filter: rg.entity_filter,
                    task_filter: rg.app_params,
                    long_entities_threshold: rg.long_entities_threshold_s.map(to_nanosecs),
                }
            }
        })
    }

    /// Populate a keyed resource timeline builder with tasks.
    fn populate_keyed_builder<'a>(
        &self,
        builder: &mut ResourceTimelineByKeyBuilder<'a, &'a str>,
        tasks: impl Iterator<Item = &'a Task>,
        resource_filter: impl Fn(Uuid) -> bool,
    ) -> AnalyzerResult<()> {
        for task in tasks {
            for (state_name, usage) in task.usages_with_state_names() {
                if resource_filter(usage.resource_id()) {
                    builder.try_push(state_name, &usage)?;
                }
            }
        }
        Ok(())
    }

    /// Turn a list of entity ids into UI-compatible FSM data.
    fn task_entities_to_ui_fsm(
        &self,
        entity_ids: &[Uuid],
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<Vec<FiniteStateMachine>> {
        entity_ids
            .iter()
            .filter_map(|&id| {
                self.model
                    .tasks
                    .get(&id)
                    .map(|task| task.try_to_ui_fsm(epoch))
            })
            .collect()
    }

    /// Convert a timeline to a UI-compatible one.
    fn timeline_to_ui(
        &self,
        result: ResourceTimeline,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<UiResourceTimeline> {
        let config = result.config.try_to_secs_relative(epoch)?;
        let capacities_values = result
            .data
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        let long_fsms = self.task_entities_to_ui_fsm(&result.long_entities, epoch)?;
        Ok(UiResourceTimeline::Binned(ResourceTimelineBinned {
            config,
            capacities_values,
            long_fsms,
        }))
    }

    /// Convert a keyed timeline to a UI-compatible one.
    fn timeline_to_ui_keyed(
        &self,
        result: ResourceTimelineByKey<&str>,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<UiResourceTimeline> {
        let config = result.config.try_to_secs_relative(epoch)?;
        let mut capacities_states_values = StdHashMap::new();
        for ((state_name, capacity_name), values) in result.data {
            capacities_states_values
                .entry(capacity_name.to_owned())
                .or_insert_with(StdHashMap::new)
                .insert(state_name.to_owned(), values);
        }
        let long_fsms = self.task_entities_to_ui_fsm(&result.long_entities, epoch)?;
        Ok(UiResourceTimeline::BinnedByState(
            ResourceTimelineBinnedByState {
                config,
                capacities_states_values,
                long_fsms,
            },
        ))
    }
}

/// Helper struct to build bulk timeline responses.
struct BulkEntryPrep<'a> {
    resource_type: &'a ResourceTypeDecl,
    resource_id_filter: HashSet<Uuid>,
    entity_filter: EntityFilter,
    task_filter: TaskFilter,
    long_entities_threshold: Option<TimeNanoSec>,
}
