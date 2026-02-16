use std::{collections::HashSet, num::NonZero};

use rustc_hash::FxHashMap as HashMap;
use std::collections::HashMap as StdHashMap;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Span,
    fsm::{Fsm, State, collection::FsmCollection},
    resource::{ResourceTypeDecl, Using, collection::ResourceCollection, tree::ResourceTreeNode},
    timeline::binned::resource::{ResourceTimelineBuilder, ResourceTimelineByKeyBuilder},
};
use quent_events::Event;
use quent_query_engine_analyzer::{self as qea};
use quent_simulator_events::SimulatorEvent;
use quent_simulator_ui::{
    self as ui, EntityRef, QueryBundle, QueryEntities, ResourceGroupNode,
    timeline::{BulkTimelineData, BulkTimelineRequestParams, BulkTimelineResponseEntry},
};
use quent_time::{SpanNanoSec, TimeUnixNanoSec, bin::BinnedSpan, to_secs};
use uuid::Uuid;

use crate::model::{SimulatorModel, SimulatorModelBuilder, SimulatorModelQueryView};

pub mod model;
pub mod task;

fn convert_ref(entity_ref: qea::EntityRef) -> EntityRef {
    match entity_ref {
        qea::EntityRef::Engine(id) => EntityRef::Engine(id),
        qea::EntityRef::Worker(id) => EntityRef::Worker(id),
        qea::EntityRef::QueryGroup(id) => EntityRef::QueryGroup(id),
        qea::EntityRef::Query(id) => EntityRef::Query(id),
        qea::EntityRef::Plan(id) => EntityRef::Plan(id),
        qea::EntityRef::Operator(id) => EntityRef::Operator(id),
        qea::EntityRef::Port(id) => EntityRef::Port(id),
    }
}

fn convert_resource_tree(
    node: ResourceTreeNode,
    view: &SimulatorModelQueryView,
) -> AnalyzerResult<Option<ui::ResourceTree>> {
    match node {
        ResourceTreeNode::ResourceGroup(id, children) => {
            let entity_ref = view
                .entity_ref(id)
                .map(convert_ref)
                .unwrap_or(EntityRef::ResourceGroup(id));
            let children: Vec<ui::ResourceTree> = children
                .into_iter()
                .map(|child| convert_resource_tree(child, view))
                .collect::<AnalyzerResult<Vec<Option<ui::ResourceTree>>>>()?
                .into_iter()
                .flatten()
                .collect();
            if !children.is_empty() {
                Ok(Some(ui::ResourceTree::ResourceGroup(ResourceGroupNode {
                    id: entity_ref,
                    children,
                })))
            } else {
                Ok(None)
            }
        }
        ResourceTreeNode::Resource(id) => {
            // Try query engine entities first, otherwise it's a simulator resource
            let entity_ref = view
                .entity_ref(id)
                .map(convert_ref)
                .unwrap_or(EntityRef::Resource(id));
            Ok(Some(ui::ResourceTree::Resource(entity_ref)))
        }
    }
}

pub struct Analyzer {
    pub model: SimulatorModel,
}

impl Analyzer {
    pub fn try_new(
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
            resources = model.resources.resources.len(),
            resource_groups = model.resources.resource_groups.len(),
            resource_types = model.resources.resource_types.len(),
            resource_group_types = model.resource_group_types.len(),
            tasks = model.tasks.len(),
        );

        Ok(Self { model })
    }

    pub fn resource_tree(&self, query_id: Uuid) -> AnalyzerResult<ResourceTreeNode> {
        let view = SimulatorModelQueryView::try_new(&self.model, query_id)?;
        ResourceTreeNode::try_new(&view, view.query_engine_view.engine.id)
    }

    pub fn query_bundle(&self, query_id: Uuid) -> AnalyzerResult<QueryBundle> {
        let qe = &self.model.query_engine;

        let start_time_unix_ns = qe.query_epoch(query_id)?;
        let duration_s = to_secs(qe.query(query_id)?.span()?.duration());

        let view = SimulatorModelQueryView::try_new(&self.model, query_id)?;
        let qe_view = &view.query_engine_view;

        let epoch = qe.query_epoch(query_id)?;

        Ok(QueryBundle {
            query_id,
            entities: QueryEntities {
                engine: qe_view.engine.try_into()?,
                query_group: qe_view.query_group.into(),
                query: qe_view.query.try_into()?,
                workers: qe_view
                    .workers
                    .values()
                    .map(|&w| (w, epoch).try_into().map(|v| (w.id(), v)))
                    .collect::<AnalyzerResult<_>>()?,
                plans: qe_view
                    .plans
                    .values()
                    .map(|&p| (p, epoch).try_into().map(|v| (p.id(), v)))
                    .collect::<AnalyzerResult<_>>()?,
                operators: qe_view
                    .operators
                    .values()
                    .map(|&o| (o, epoch).try_into().map(|v| (o.id(), v)))
                    .collect::<AnalyzerResult<_>>()?,
                ports: qe_view
                    .ports
                    .values()
                    .map(|&p| (p, epoch).try_into().map(|v| (p.id(), v)))
                    .collect::<AnalyzerResult<_>>()?,
                resources: self
                    .model
                    .resources
                    .resources()
                    .map(|res| (res.id(), res.into()))
                    .collect(),
                resource_types: self
                    .model
                    .resources
                    .resource_types
                    .iter()
                    .map(|(k, v)| (k.clone(), v.into()))
                    .collect(),
                resource_groups: self
                    .model
                    .resources
                    .resource_groups()
                    .map(|res| (res.id(), res.into()))
                    .collect(),
                resource_group_types: self
                    .model
                    .resource_group_types
                    .iter()
                    .map(|(k, v)| (k.clone(), v.into()))
                    .collect(),
            },
            plan_tree: (&self.model.query_engine.plan_tree(query_id)?).into(),
            resource_tree: convert_resource_tree(self.resource_tree(query_id)?, &view)?
                .ok_or_else(|| {
                    AnalyzerError::Validation(format!(
                        "query {query_id} has no top-level resource group"
                    ))
                })?,
            unique_operator_names: qe_view
                .operators
                .values()
                .filter_map(|v| v.operator_type_name.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),
            start_time_unix_ns,
            duration_s,
        })
    }

    pub fn resource_timeline(
        &self,
        resource_id: Uuid,
        fsm_type_name: Option<String>,
        operator_id: Option<Uuid>,
        config: BinnedSpan,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<ui::timeline::TimelineResponse> {
        // Sanity check the resource id.
        let resource_type = self.model.resource_type_of(resource_id)?;

        let filtered_tasks = || {
            self.model
                .tasks
                .iter()
                .filter(move |task| operator_id.is_none_or(|op| task.operator_id() == Some(op)))
        };

        // If an FSM type name was supplied, we will deliver a timeline split
        // out over states of that FSM.
        if let Some(fsm_type_name) = fsm_type_name {
            // Sanity check the fsm type exists.
            if !self.model.contains_fsm_type(&fsm_type_name) {
                Err(AnalyzerError::InvalidTypeName(fsm_type_name))
            } else {
                let mut builder =
                    ResourceTimelineByKeyBuilder::try_new(resource_type, resource_id, config)?;
                match fsm_type_name.as_str() {
                    "task" => {
                        builder.try_extend(filtered_tasks().flat_map(|task| {
                            task.states().flat_map(|state| {
                                state
                                    .usages()
                                    .map(|(usage, span)| (state.name(), usage, span))
                            })
                        }))?;
                    }
                    _ => unimplemented!(),
                }
                let result = builder.build();
                let config_secs = result.config.try_to_secs_relative(epoch)?;
                let mut capacities_states_values = StdHashMap::new();
                for ((state_name, capacity_name), values) in result.data {
                    capacities_states_values
                        .entry(capacity_name.to_owned())
                        .or_insert_with(StdHashMap::new)
                        .insert(state_name.to_owned(), values);
                }
                Ok(ui::timeline::TimelineResponse::BinnedByState(
                    ui::timeline::ResourceTimelineBinnedByState {
                        config: config_secs,
                        capacities_states_values,
                    },
                ))
            }
        } else {
            let mut builder = ResourceTimelineBuilder::try_new(resource_type, resource_id, config)?;
            builder.try_extend(filtered_tasks().flat_map(|task| task.usages()))?;
            let result = builder.build();
            let config_secs = result.config.try_to_secs_relative(epoch)?;
            let capacities_values = result
                .data
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect();
            Ok(ui::timeline::TimelineResponse::Binned(
                ui::timeline::ResourceTimelineBinned {
                    config: config_secs,
                    capacities_values,
                },
            ))
        }
    }

    pub fn resource_group_timeline(
        &self,
        resource_group_id: Uuid,
        resource_type_name: &str,
        fsm_type_name: Option<String>,
        operator_id: Option<Uuid>,
        config: BinnedSpan,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<ui::timeline::TimelineResponse> {
        // Sanity check the resource group exists
        self.model.resource_group(resource_group_id)?;

        // Get the resource type
        let resource_type = self.model.resources.resource_type(resource_type_name)?;

        // Build the resource tree for this group
        let tree = ResourceTreeNode::try_new(&self.model, resource_group_id)?;

        // Collect all leaf resource IDs of the requested type in the tree
        let resource_ids: HashSet<Uuid> = tree
            .iter_leaf_ids()
            .filter(|&id| {
                self.model
                    .resource(id)
                    .ok()
                    .map(|r| r.type_name() == resource_type_name)
                    .unwrap_or(false)
            })
            .collect();

        let filtered_tasks = || {
            self.model
                .tasks
                .iter()
                .filter(move |task| operator_id.is_none_or(|op| task.operator_id() == Some(op)))
        };

        // If an FSM type name was supplied, we will deliver a timeline split
        // out over states of that FSM.
        if let Some(fsm_type_name) = fsm_type_name {
            // Sanity check the fsm type exists.
            if !self.model.contains_fsm_type(&fsm_type_name) {
                Err(AnalyzerError::InvalidTypeName(fsm_type_name))
            } else {
                let mut builder =
                    ResourceTimelineByKeyBuilder::try_new(resource_type, resource_ids, config)?;
                match fsm_type_name.as_str() {
                    "task" => {
                        builder.try_extend(filtered_tasks().flat_map(|task| {
                            task.states().flat_map(|state| {
                                state
                                    .usages()
                                    .map(|(usage, span)| (state.name(), usage, span))
                            })
                        }))?;
                    }
                    _ => unimplemented!(),
                }
                let result = builder.build();
                let config_secs = result.config.try_to_secs_relative(epoch)?;
                let mut capacities_states_values = StdHashMap::new();
                for ((state_name, capacity_name), values) in result.data {
                    capacities_states_values
                        .entry(capacity_name.to_owned())
                        .or_insert_with(StdHashMap::new)
                        .insert(state_name.to_owned(), values);
                }
                Ok(ui::timeline::TimelineResponse::BinnedByState(
                    ui::timeline::ResourceTimelineBinnedByState {
                        config: config_secs,
                        capacities_states_values,
                    },
                ))
            }
        } else {
            let mut builder =
                ResourceTimelineBuilder::try_new(resource_type, resource_ids, config)?;
            builder.try_extend(filtered_tasks().flat_map(|task| task.usages()))?;
            let result = builder.build();
            let config_secs = result.config.try_to_secs_relative(epoch)?;
            let capacities_values = result
                .data
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect();
            Ok(ui::timeline::TimelineResponse::Binned(
                ui::timeline::ResourceTimelineBinned {
                    config: config_secs,
                    capacities_values,
                },
            ))
        }
    }

    /// Resolve the resource type and id filter for a bulk timeline entry.
    fn try_prepare_bulk_entry<'a>(
        &'a self,
        resource_id: Uuid,
        tree: &ResourceTreeNode,
        params: &ui::timeline::BulkTimelineRequestParams,
    ) -> AnalyzerResult<BulkEntryBuilder<'a>> {
        let (resource_type, resource_id_filter, fsm_type_name, operator_id) = match params {
            BulkTimelineRequestParams::Resource(p) => {
                let resource_type = self.model.resource_type_of(resource_id)?;
                (
                    resource_type,
                    [resource_id].into(),
                    p.fsm_type_name.as_deref(),
                    p.operator_id,
                )
            }
            BulkTimelineRequestParams::ResourceGroup(p) => {
                let resource_type = self.model.resources.resource_type(&p.resource_type_name)?;
                let resource_group_id = resource_id;
                let subtree = tree
                    .find(resource_group_id)
                    .ok_or(AnalyzerError::InvalidId(resource_group_id))?;
                let id_filter: HashSet<Uuid> = subtree
                    .iter_leaf_ids()
                    .filter(|&id| {
                        self.model
                            .resource(id)
                            .ok()
                            .is_some_and(|r| r.type_name() == p.resource_type_name)
                    })
                    .collect();
                (
                    resource_type,
                    id_filter,
                    p.fsm_type_name.as_deref(),
                    p.operator_id,
                )
            }
        };

        let keyed = if let Some(fsm_type_name) = fsm_type_name {
            if !self.model.contains_fsm_type(fsm_type_name) {
                return Err(AnalyzerError::InvalidTypeName(fsm_type_name.to_owned()));
            }
            true
        } else {
            false
        };

        Ok(BulkEntryBuilder {
            resource_type,
            id_filter: resource_id_filter,
            per_state: keyed,
            operator_id,
        })
    }

    pub fn bulk_timelines(
        &self,
        query_id: Uuid,
        request: ui::timeline::BulkTimelinesRequest,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<ui::timeline::BulkTimelinesResponse> {
        let span = SpanNanoSec::try_new(
            epoch + (request.start * 1e9) as u64,
            epoch + (request.end * 1e9) as u64,
        )?;
        // Validate the config params
        let num_bins = NonZero::<u64>::try_from(request.num_bins as u64)
            .map_err(|e| AnalyzerError::InvalidArgument(format!("num_bins must be > 0: {e}")))?;
        let config = BinnedSpan::try_new(span, num_bins)?;
        let config_secs = config.try_to_secs_relative(epoch)?;

        // Prepare builders
        let mut plain_builders: Vec<(Uuid, ResourceTimelineBuilder<HashSet<Uuid>>)> = Vec::new();
        let mut plain_op_filters: Vec<Option<Uuid>> = Vec::new();
        let mut per_state_builders: Vec<(Uuid, ResourceTimelineByKeyBuilder<HashSet<Uuid>, &str>)> =
            Vec::new();
        let mut per_state_op_filters: Vec<Option<Uuid>> = Vec::new();

        // Prepare resource tree, we'll re-use this as it may be expensive to
        // build for every entry.
        let resource_tree = self.resource_tree(query_id)?;

        for (resource_id, params) in &request.entries {
            let entry = self.try_prepare_bulk_entry(*resource_id, &resource_tree, params)?;
            if entry.per_state {
                per_state_op_filters.push(entry.operator_id);
                per_state_builders.push((
                    *resource_id,
                    ResourceTimelineByKeyBuilder::try_new(
                        entry.resource_type,
                        entry.id_filter,
                        config,
                    )?,
                ));
            } else {
                plain_op_filters.push(entry.operator_id);
                plain_builders.push((
                    *resource_id,
                    ResourceTimelineBuilder::try_new(entry.resource_type, entry.id_filter, config)?,
                ));
            }
        }

        // Build reverse index of resource_(group)_id -> list of builder indices
        // into which we'll push usages.
        let plain_index: HashMap<Uuid, Vec<usize>> = plain_builders
            .iter()
            .enumerate()
            .flat_map(|(i, (_, builder))| builder.id_filter().iter().map(move |&id| (id, i)))
            .fold(HashMap::default(), |mut acc, (id, i)| {
                acc.entry(id).or_default().push(i);
                acc
            });

        let per_state_index: HashMap<Uuid, Vec<usize>> = per_state_builders
            .iter()
            .enumerate()
            .flat_map(|(i, (_, builder))| builder.id_filter().iter().map(move |&id| (id, i)))
            .fold(HashMap::default(), |mut acc, (id, i)| {
                acc.entry(id).or_default().push(i);
                acc
            });

        // Iterate over all task usages once, routing each usage to matching builders
        for task in self.model.tasks.iter() {
            let task_operator_id = task.operator_id();
            for state in task.states() {
                for (usage, span) in state.usages() {
                    if let Some(indices) = plain_index.get(&usage.resource) {
                        for &i in indices {
                            if plain_op_filters[i].is_none_or(|op| task_operator_id == Some(op)) {
                                plain_builders[i].1.try_push_prefiltered(usage, span)?;
                            }
                        }
                    }
                    if let Some(indices) = per_state_index.get(&usage.resource) {
                        for &i in indices {
                            if per_state_op_filters[i].is_none_or(|op| task_operator_id == Some(op))
                            {
                                per_state_builders[i].1.try_push_prefiltered(
                                    state.name(),
                                    usage,
                                    span,
                                )?;
                            }
                        }
                    }
                }
            }
        }

        // Collect results.
        let mut result = HashMap::default();
        for (key, builder) in plain_builders {
            result.insert(
                key,
                BulkTimelineResponseEntry::Ok {
                    message: String::new(),
                    data: BulkTimelineData::Binned {
                        capacities_values: builder
                            .build()
                            .data
                            .into_iter()
                            .map(|(k, v)| (k.to_owned(), v))
                            .collect(),
                    },
                },
            );
        }

        for (key, builder) in per_state_builders {
            let mut capacities_states_values = std::collections::HashMap::default();
            // Unflatten the keys
            for ((state_name, capacity_name), values) in builder.build().data {
                capacities_states_values
                    .entry(capacity_name.to_owned())
                    .or_insert_with(std::collections::HashMap::new)
                    .insert(state_name.to_owned(), values);
            }
            result.insert(
                key,
                BulkTimelineResponseEntry::Ok {
                    message: String::new(),
                    data: BulkTimelineData::BinnedByState {
                        capacities_states_values,
                    },
                },
            );
        }

        Ok(ui::timeline::BulkTimelinesResponse {
            config: config_secs,
            resources: result
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        })
    }
}

/// Helper struct to build bulk timeline responses.
struct BulkEntryBuilder<'a> {
    resource_type: &'a ResourceTypeDecl,
    id_filter: HashSet<Uuid>,
    per_state: bool,
    operator_id: Option<Uuid>,
}
