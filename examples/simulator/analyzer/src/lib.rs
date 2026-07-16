// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::Event;
pub use quent_query_engine_analyzer::QueryEngineModel;
use quent_query_engine_analyzer::{
    entities,
    ui::{QuentViewer, UiAnalyzer, ViewerEventStream},
};
use quent_query_engine_ui::{
    DataFlowTimelineBinned, DataFlowTimelineResponse, OperatorFilter, QueryBundle, QueryEntities,
    QueryFilter,
};
use quent_ui::{
    FiniteStateMachine, ResourceGroupNode, ResourceTree, convert_resource_tree,
    quantity::{CapacityKind, QuantitySpec},
    timeline::{
        distribution::{
            DimensionKeyDecl, DistributionDecl, DistributionSeries, DistributionTimelineRequest,
            MeasureDecl,
        },
        request::{
            BulkChunkedTimelineRequest, BulkTimelineRequest, EntityFilter, SingleTimelineRequest,
            TimelineRequest,
        },
        response::{
            BulkChunkedTimelinesResponse, BulkTimelinesResponse, BulkTimelinesResponseEntry,
            ResourceTimeline as UiResourceTimeline, ResourceTimelineBinned,
            ResourceTimelineBinnedByState, SingleTimelineResponse,
        },
    },
};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::collections::HashMap as StdHashMap;
use std::sync::Arc;
use tracing::debug;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Model, Span,
    fsm::{FsmTypeDeclaration, FsmUsages, Transition},
    resource::{
        ResourceTypeDecl, Usage, Using, collection::ResourceCollection, tree::ResourceTreeNode,
    },
    timeline::binned::{
        distribution::{DistributionKey, DistributionTimelineBuilder},
        resource::{
            ResourceTimeline, ResourceTimelineBuilder, ResourceTimelineByKey,
            ResourceTimelineByKeyBuilder,
        },
    },
};
use quent_simulator_instrumentation::{Simulator, SimulatorEvent};
use quent_simulator_ui::EntityRef;
use quent_time::{SpanNanoSec, TimeNanoSec, TimeUnixNanoSec, Timestamp, to_nanosecs, to_secs};
use uuid::Uuid;

use crate::{
    model::{SimulatorModel, SimulatorModelBuilder},
    task::{Task, TaskExt},
};

pub mod model;
pub mod task;
pub mod view;

/// Data-flow measure counting tasks residing in each (state, location) cell.
const MEASURE_TASKS: &str = "tasks";
/// Data-flow measure summing memory bytes held in each (state, location) cell.
const MEASURE_BYTES: &str = "bytes";
/// Data-flow dimension key for states that hold no memory resource.
const DIMENSION_NONE: &str = "none";
/// Type name of stdlib memory resources as recorded by the model.
const MEMORY_TYPE_NAME: &str = "memory";

pub struct SimulatorUiAnalyzer {
    pub model: SimulatorModel,
}

/// `quent-open` viewer entry for the simulator model: renders [`SimulatorEvent`]
/// streams with [`SimulatorUiAnalyzer`]. The required `Viewer` path
/// `quent-open` names when building a viewer for this analyzer's models.
pub struct Viewer;

impl QuentViewer for Viewer {
    type Analyzer = SimulatorUiAnalyzer;

    fn import_events(
        dir: &std::path::Path,
    ) -> quent_model::io::ImporterResult<ViewerEventStream<Self::Analyzer>> {
        Simulator::import_events(dir)
    }
}

struct PlainBuilderSlot<'a> {
    entry_id: String,
    config_idx: usize,
    builder: ResourceTimelineBuilder<'a>,
    resource_id_filter: Arc<HashSet<Uuid>>,
    operator_filter: OperatorFilter,
}

struct PerStateBuilderSlot<'a> {
    entry_id: String,
    config_idx: usize,
    builder: ResourceTimelineByKeyBuilder<'a, &'a str>,
    resource_id_filter: Arc<HashSet<Uuid>>,
    operator_filter: OperatorFilter,
}

impl UiAnalyzer for SimulatorUiAnalyzer {
    type Event = SimulatorEvent;
    type EntityRef = EntityRef;

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
                ("capacity_bytes".into(), QuantitySpec::bytes()),
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

    fn list_entities(
        &self,
        request: quent_ui::entities::request::EntityListRequest<QueryFilter, OperatorFilter>,
    ) -> AnalyzerResult<quent_ui::entities::response::EntityListResponse> {
        let query_id = request.app_params.query_id;
        let epoch = self.query_engine_model().query_epoch(query_id)?;
        let entry = request.entry;
        let window = entry.window.try_into_span(epoch)?;
        let scope = entry
            .filter
            .scope
            .as_ref()
            .map(|s| s.resolve(&self.model))
            .transpose()?;
        let operator_filter = entry.application.operator_id;

        // Restrict candidates to the requested query: a task belongs to a query
        // iff its operator is one of that query's operators. Without this, tasks
        // from a different query sharing a resource and overlapping the window
        // would leak in.
        let query_operators: HashSet<Uuid> = self
            .model
            .query_view(query_id)?
            .operators()
            .map(|op| op.id())
            .collect();

        entities::list_entities(
            &self.model,
            |task| {
                task.operator_id().is_some_and(|op| {
                    query_operators.contains(&op)
                        && operator_filter.is_none_or(|filter| op == filter)
                })
            },
            entities::ListQuery {
                scope: scope.as_ref(),
                window,
                filter: &entry.filter,
                sort: entry.sort,
                page: entry.page,
                epoch,
            },
        )
    }

    // TODO(johanpel): consider re-using the bulk request API with a single entry for requests like this.
    fn single_resource_timeline(
        &self,
        request: SingleTimelineRequest<QueryFilter, OperatorFilter>,
    ) -> AnalyzerResult<SingleTimelineResponse> {
        // TODO(johanpel): we may want to sanity-check whether the requested
        // resource/group is actually in the resource tree for a given query.

        // Calculate this ASAP to help fail quickly.
        let epoch = self
            .query_engine_model()
            .query_epoch(request.app_params.query_id)?;
        let config = request.entry.config().try_into_binned_span(epoch)?;
        let config_secs = config.try_to_secs_relative(epoch)?;

        match request.entry {
            TimelineRequest::Resource(req) => {
                let resource_type = self.model.resource_type_of(req.resource_id)?;
                let long_entities_threshold = req.long_entities_threshold_s.map(to_nanosecs);
                let operator_filter = req.application;

                if req.entity_filter.entity_type_name.is_some() {
                    let mut builder = ResourceTimelineByKeyBuilder::try_new(
                        resource_type,
                        config,
                        long_entities_threshold,
                    )?;
                    // This application only has Task FSM
                    self.populate_keyed_builder(
                        &mut builder,
                        self.entities_filtered(req.entity_filter, operator_filter, config.span)?
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
                        self.entities_filtered(req.entity_filter, operator_filter, config.span)?
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
        request: BulkTimelineRequest<QueryFilter, OperatorFilter>,
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
        let mut plain_builders: Vec<(
            String,
            ResourceTimelineBuilder,
            HashSet<Uuid>,
            OperatorFilter,
        )> = Vec::new();

        // Prepare them also for keyed builders (building by state).
        let mut per_state_builders: Vec<(
            String,
            ResourceTimelineByKeyBuilder<&str>,
            HashSet<Uuid>,
            OperatorFilter,
        )> = Vec::new();

        for (entry_id, entry) in request.entries {
            let entry_config = entry.config().try_into_binned_span(epoch)?;
            let BulkEntryPrep {
                resource_type,
                resource_id_filter,
                entity_filter,
                operator_filter,
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
                    operator_filter,
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
                    operator_filter,
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

    fn bulk_chunked_resource_timeline(
        &self,
        request: BulkChunkedTimelineRequest<QueryFilter, OperatorFilter>,
    ) -> AnalyzerResult<BulkChunkedTimelinesResponse> {
        let epoch = self
            .query_engine_model()
            .query_epoch(request.app_params.query_id)?;
        let view = self.model.query_view(request.app_params.query_id)?;
        let resource_tree = view.resource_tree()?;

        let n_configs = request.configs.len();

        let mut plain_builders: Vec<PlainBuilderSlot<'_>> =
            Vec::with_capacity(request.entries.len() * n_configs);
        let mut per_state_builders: Vec<PerStateBuilderSlot<'_>> =
            Vec::with_capacity(request.entries.len() * n_configs);

        // Per-entry prep runs once; the builders for that entry's N configs all share it.
        for (entry_id, entry) in &request.entries {
            let BulkEntryPrep {
                resource_type,
                resource_id_filter,
                entity_filter,
                operator_filter,
                long_entities_threshold,
            } = self.try_prepare_bulk_entry(entry.clone(), &resource_tree)?;

            // Wrap the filter once so per-config slots share one allocation.
            let resource_id_filter = Arc::new(resource_id_filter);

            for (config_idx, config) in request.configs.iter().enumerate() {
                let entry_config = config.try_into_binned_span(epoch)?;
                if entity_filter.entity_type_name.is_some() {
                    per_state_builders.push(PerStateBuilderSlot {
                        entry_id: entry_id.clone(),
                        config_idx,
                        builder: ResourceTimelineByKeyBuilder::try_new(
                            resource_type,
                            entry_config,
                            long_entities_threshold,
                        )?,
                        resource_id_filter: Arc::clone(&resource_id_filter),
                        operator_filter: operator_filter.clone(),
                    });
                } else {
                    plain_builders.push(PlainBuilderSlot {
                        entry_id: entry_id.clone(),
                        config_idx,
                        builder: ResourceTimelineBuilder::try_new(
                            resource_type,
                            entry_config,
                            long_entities_threshold,
                        )?,
                        resource_id_filter: Arc::clone(&resource_id_filter),
                        operator_filter: operator_filter.clone(),
                    });
                }
            }
        }

        let plain_index: HashMap<Uuid, Vec<usize>> = plain_builders
            .iter()
            .enumerate()
            .flat_map(|(builder_idx, slot)| {
                slot.resource_id_filter
                    .iter()
                    .map(move |&resource_id| (resource_id, builder_idx))
            })
            .fold(HashMap::default(), |mut acc, (resource_id, builder_idx)| {
                acc.entry(resource_id).or_default().push(builder_idx);
                acc
            });
        let per_state_index: HashMap<Uuid, Vec<usize>> = per_state_builders
            .iter()
            .enumerate()
            .flat_map(|(builder_idx, slot)| {
                slot.resource_id_filter
                    .iter()
                    .map(move |&resource_id| (resource_id, builder_idx))
            })
            .fold(HashMap::default(), |mut acc, (resource_id, builder_idx)| {
                acc.entry(resource_id).or_default().push(builder_idx);
                acc
            });

        // Single pass over all tasks/usages — the dominant cost — dispatched to
        // every matching (entry, config) builder. Builders filter by their own
        // span internally, so out-of-window usages are no-ops.
        for task in self.model.tasks.values() {
            let task_operator_id = task.operator_id();
            for usage in task.usages() {
                let resource_id = usage.resource_id();
                if let Some(builder_indices) = plain_index.get(&resource_id) {
                    for &builder_idx in builder_indices {
                        let slot = &plain_builders[builder_idx];
                        if slot
                            .operator_filter
                            .operator_id
                            .is_none_or(|op| task_operator_id == Some(op))
                        {
                            plain_builders[builder_idx].builder.try_push(&usage)?;
                        }
                    }
                }
            }
            for (state_name, usage) in task.usages_with_state_names() {
                let resource_id = usage.resource_id();
                if let Some(builder_indices) = per_state_index.get(&resource_id) {
                    for &builder_idx in builder_indices {
                        let slot = &per_state_builders[builder_idx];
                        if slot
                            .operator_filter
                            .operator_id
                            .is_none_or(|op| task_operator_id == Some(op))
                        {
                            per_state_builders[builder_idx]
                                .builder
                                .try_push(state_name, &usage)?;
                        }
                    }
                }
            }
        }

        // Reassemble per-entry Vec aligned with `request.configs` order. Slots
        // start as `None` and must all be filled by the end — every (entry,
        // config_idx) had a builder, and every builder produces an `Ok`.
        let mut slots: HashMap<String, Vec<Option<BulkTimelinesResponseEntry>>> = request
            .entries
            .keys()
            .map(|k| (k.clone(), (0..n_configs).map(|_| None).collect()))
            .collect();

        for slot in plain_builders {
            let built = slot.builder.build();
            let config = built.config.try_to_secs_relative(epoch)?;
            let resp = BulkTimelinesResponseEntry::Ok {
                message: String::new(),
                config,
                data: self.timeline_to_ui(built, epoch)?,
            };
            slots.get_mut(&slot.entry_id).unwrap_or_else(|| {
                panic!("known key, instead found unknown key {}", slot.entry_id)
            })[slot.config_idx] = Some(resp);
        }
        for slot in per_state_builders {
            let built = slot.builder.build();
            let config = built.config.try_to_secs_relative(epoch)?;
            let resp = BulkTimelinesResponseEntry::Ok {
                message: String::new(),
                config,
                data: self.timeline_to_ui_keyed(built, epoch)?,
            };
            slots.get_mut(&slot.entry_id).unwrap_or_else(|| {
                panic!("known key, instead found unknown key {}", slot.entry_id)
            })[slot.config_idx] = Some(resp);
        }

        let entries = slots
            .into_iter()
            .map(|(k, v)| {
                let v = v
                    .into_iter()
                    .map(|opt| {
                        opt.ok_or(AnalyzerError::BrokenImpl(
                            "chunked bulk: missing builder slot",
                        ))
                    })
                    .collect::<AnalyzerResult<Vec<_>>>()?;
                Ok((k, v))
            })
            .collect::<AnalyzerResult<std::collections::HashMap<_, _>>>()?;

        Ok(BulkChunkedTimelinesResponse { entries })
    }

    fn data_flow_timeline(
        &self,
        request: DistributionTimelineRequest<QueryFilter>,
    ) -> AnalyzerResult<DataFlowTimelineResponse> {
        let query_id = request.app_params.query_id;
        let epoch = self.query_engine_model().query_epoch(query_id)?;
        let config = request.config.try_into_binned_span(epoch)?;

        // Which of the declared measures to compute; empty means all.
        let want =
            |name: &str| request.measures.is_empty() || request.measures.iter().any(|m| m == name);
        let want_tasks = want(MEASURE_TASKS);
        let want_bytes = want(MEASURE_BYTES);
        if !want_tasks && !want_bytes {
            return Err(AnalyzerError::InvalidArgument(format!(
                "unknown measures {:?}; declared measures are '{MEASURE_TASKS}' and '{MEASURE_BYTES}'",
                request.measures
            )));
        }

        let query_operators: HashSet<Uuid> = self
            .model
            .query_view(query_id)?
            .operators()
            .map(|op| op.id())
            .collect();

        // The dimension of the distribution is where a task's data resides:
        // the instance name of the memory-typed resource its state uses, or
        // `DIMENSION_NONE` for states that hold no memory.
        let memory_names: HashMap<Uuid, &str> = self
            .model
            .arbitrary_resources
            .resources()
            .filter(|r| r.type_name() == MEMORY_TYPE_NAME)
            .map(|r| (r.id(), r.instance_name()))
            .collect();

        // The no-memory sentinel must never collide with a real resource
        // name; grow it until it is unique among memory instance names.
        let mut none_key = DIMENSION_NONE.to_owned();
        while memory_names.values().any(|name| *name == none_key) {
            none_key.push('_');
        }
        // Dimension keys actually observed for this query's tasks; the decl
        // advertises only these (not every memory in the engine model).
        let mut present_dimensions: HashSet<&str> = HashSet::default();

        let mut builder = DistributionTimelineBuilder::<Uuid>::new(config);
        for task in self.model.tasks.values() {
            let Some(operator_id) = task.operator_id() else {
                continue;
            };
            if !query_operators.contains(&operator_id) {
                continue;
            }
            // Walk state spans: state `i` spans transition `i` to `i + 1`. Use
            // raw transitions rather than `usages_with_state_names` so states
            // without usages still count.
            for pair in task.transitions().windows(2) {
                let (from, to) = (&pair[0], &pair[1]);
                let Ok(span) = SpanNanoSec::try_new(from.timestamp(), to.timestamp()) else {
                    continue;
                };
                let state = from.name();
                let memory_usage = from
                    .usages
                    .iter()
                    .find(|u| memory_names.contains_key(&u.resource_id));
                let dimension =
                    memory_usage.map_or(none_key.as_str(), |u| memory_names[&u.resource_id]);
                if want_tasks {
                    present_dimensions.insert(dimension);
                    builder.try_push(
                        DistributionKey {
                            series: operator_id,
                            measure: MEASURE_TASKS,
                            state,
                            dimension,
                        },
                        span,
                        1.0,
                    )?;
                }
                if want_bytes {
                    let bytes: u64 = memory_usage
                        .map(|u| {
                            u.capacities
                                .iter()
                                .filter(|c| c.name == "capacity_bytes")
                                .filter_map(|c| c.value)
                                .sum()
                        })
                        .unwrap_or(0);
                    if bytes > 0 {
                        present_dimensions.insert(dimension);
                        builder.try_push(
                            DistributionKey {
                                series: operator_id,
                                measure: MEASURE_BYTES,
                                state,
                                dimension,
                            },
                            span,
                            bytes as f64,
                        )?;
                    }
                }
            }
        }

        // Pivot the flat aggregation into per-operator nested series. All-zero
        // series (e.g. from zero-duration states) are omitted; the protocol
        // treats absent entries as all-zero bins.
        let mut operators: StdHashMap<Uuid, DistributionSeries> = StdHashMap::new();
        for (key, bins) in builder.build().data {
            if bins.iter().all(|v| *v == 0.0) {
                continue;
            }
            operators
                .entry(key.series)
                .or_default()
                .values
                .entry(key.measure.to_owned())
                .or_default()
                .entry(key.state.to_owned())
                .or_default()
                .insert(key.dimension.to_owned(), bins);
        }

        let has_none = present_dimensions.remove(none_key.as_str());
        let mut memory_instance_names: Vec<&str> = present_dimensions.into_iter().collect();
        memory_instance_names.sort_unstable();
        let mut dimension_keys: Vec<DimensionKeyDecl> = memory_instance_names
            .into_iter()
            .map(|name| DimensionKeyDecl {
                key: name.to_owned(),
                display_name: name.to_owned(),
            })
            .collect();
        if has_none {
            dimension_keys.push(DimensionKeyDecl {
                key: none_key.clone(),
                display_name: "No data resident".to_owned(),
            });
        }

        let mut measures = Vec::new();
        if want_tasks {
            measures.push(MeasureDecl {
                name: MEASURE_TASKS.to_owned(),
                display_name: "Tasks".to_owned(),
                quantity: "unit".to_owned(),
                kind: CapacityKind::Occupancy,
            });
        }
        if want_bytes {
            measures.push(MeasureDecl {
                name: MEASURE_BYTES.to_owned(),
                display_name: "Resident bytes".to_owned(),
                quantity: "capacity_bytes".to_owned(),
                kind: CapacityKind::Occupancy,
            });
        }

        Ok(DataFlowTimelineResponse::Binned(DataFlowTimelineBinned {
            config: config.try_to_secs_relative(epoch)?,
            decl: DistributionDecl {
                entity_type_name: Task::fsm_type_declaration().name,
                dimension_name: "Data location".to_owned(),
                dimension_keys,
                measures,
                default_measure: None,
            },
            operators,
        }))
    }
}

impl SimulatorUiAnalyzer {
    /// Return an iterator over all tasks, filtered by time window and operator id.
    fn entities_filtered(
        &self,
        entity_filter: EntityFilter,
        operator_filter: OperatorFilter,
        time_window: SpanNanoSec,
    ) -> AnalyzerResult<Box<dyn Iterator<Item = &Task> + '_>> {
        if let Some(entity_type_name) = entity_filter.entity_type_name {
            match entity_type_name.as_str() {
                "task" => Ok(Box::new(self.model.tasks.values().filter(move |task| {
                    operator_filter
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
                operator_filter
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
        request: TimelineRequest<OperatorFilter>,
        tree: &ResourceTreeNode,
    ) -> AnalyzerResult<BulkEntryPrep<'a>> {
        Ok(match request {
            TimelineRequest::Resource(r) => BulkEntryPrep {
                resource_type: self.model.resource_type_of(r.resource_id)?,
                resource_id_filter: [r.resource_id].into_iter().collect(),
                entity_filter: r.entity_filter,
                operator_filter: r.application,
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
                    operator_filter: rg.app_params,
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
    operator_filter: OperatorFilter,
    long_entities_threshold: Option<TimeNanoSec>,
}
