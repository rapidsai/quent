use std::num::NonZero;

use quent_ui::FiniteStateMachine;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::collections::HashMap as StdHashMap;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Span,
    fsm::{FsmTypeDeclaration, FsmUsages, collection::FsmCollection},
    resource::{
        ResourceTypeDecl, Usage, Using, collection::ResourceCollection, tree::ResourceTreeNode,
    },
    timeline::binned::resource::{
        ResourceTimeline, ResourceTimelineBuilder, ResourceTimelineByKey,
        ResourceTimelineByKeyBuilder,
    },
};
use quent_events::Event;
use quent_query_engine_analyzer::{self as qea};
use quent_simulator_events::SimulatorEvent;
use quent_simulator_ui::{
    self as ui, EntityRef, QueryBundle, QueryEntities, ResourceGroupNode,
    timeline::{BulkTimelineData, BulkTimelineRequestParams, BulkTimelineResponseEntry},
};
use quent_time::{
    SpanNanoSec, TimeNanoSec, TimeUnixNanoSec, bin::BinnedSpan, to_nanosecs, to_secs,
};
use uuid::Uuid;

use crate::{
    model::{SimulatorModel, SimulatorModelBuilder, SimulatorModelQueryView},
    task::Task,
};

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

        let task_decl = Task::fsm_type_declaration();

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
                fsm_types: [(task_decl.name.clone(), task_decl)].into_iter().collect(),
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

    /// Return an iterator over all tasks, filtered by time window and operator id.
    fn tasks_filtered(
        &self,
        operator_id: Option<Uuid>,
        time_window: SpanNanoSec,
    ) -> impl Iterator<Item = &Task> {
        self.model.tasks.values().filter(move |task| {
            operator_id.is_none_or(|op| task.operator_id() == Some(op))
                && task.span().is_ok_and(|s| s.intersects(&time_window))
        })
    }

    // TODO(johanpel): consider re-using the bulk request API with a single entry for requests like this.
    #[allow(clippy::too_many_arguments)]
    pub fn resource_timeline(
        &self,
        resource_id: Uuid,
        fsm_type_name: Option<String>,
        operator_id: Option<Uuid>,
        config: BinnedSpan,
        long_entities_threshold: Option<TimeNanoSec>,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<ui::timeline::TimelineResponse> {
        // Sanity check the resource id.
        let resource_type = self.model.resource_type_of(resource_id)?;

        // If an FSM type name was supplied, we will deliver a timeline split
        // out over states of that FSM.
        if let Some(fsm_type_name) = fsm_type_name {
            // Sanity check the fsm type exists.
            if !self.model.contains_fsm_type(&fsm_type_name) {
                Err(AnalyzerError::InvalidTypeName(fsm_type_name))
            } else {
                let mut builder = ResourceTimelineByKeyBuilder::try_new(
                    resource_type,
                    config,
                    long_entities_threshold,
                )?;
                if fsm_type_name.as_str() == "task" {
                    self.populate_keyed_builder(
                        &mut builder,
                        self.tasks_filtered(operator_id, config.span)
                            .filter(|task| {
                                task.usages()
                                    .any(|usage| usage.resource_id() == resource_id)
                            }),
                        |id| id == resource_id,
                    )?;
                }
                self.timeline_to_ui_keyed(builder.build(), epoch)
            }
        } else {
            let mut builder =
                ResourceTimelineBuilder::try_new(resource_type, config, long_entities_threshold)?;
            builder.try_extend(
                self.tasks_filtered(operator_id, config.span)
                    .flat_map(|task| task.usages())
                    .filter(|usage| usage.resource_id() == resource_id),
            )?;
            self.timeline_to_ui(builder.build(), epoch)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resource_group_timeline(
        &self,
        resource_group_id: Uuid,
        resource_type_name: &str,
        fsm_type_name: Option<String>,
        operator_id: Option<Uuid>,
        config: BinnedSpan,
        long_entities_threshold: Option<TimeNanoSec>,
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

        // If an FSM type name was supplied, we will deliver a timeline split
        // out over states of that FSM.
        if let Some(fsm_type_name) = fsm_type_name {
            // Sanity check the fsm type exists.
            if !self.model.contains_fsm_type(&fsm_type_name) {
                Err(AnalyzerError::InvalidTypeName(fsm_type_name))
            } else {
                let mut builder = ResourceTimelineByKeyBuilder::try_new(
                    resource_type,
                    config,
                    long_entities_threshold,
                )?;
                if fsm_type_name.as_str() == "task" {
                    self.populate_keyed_builder(
                        &mut builder,
                        self.tasks_filtered(operator_id, config.span)
                            .filter(|task| {
                                task.usages()
                                    .any(|usage| resource_ids.contains(&usage.resource_id()))
                            }),
                        |id| resource_ids.contains(&id),
                    )?;
                }
                self.timeline_to_ui_keyed(builder.build(), epoch)
            }
        } else {
            let mut builder =
                ResourceTimelineBuilder::try_new(resource_type, config, long_entities_threshold)?;
            builder.try_extend(
                self.tasks_filtered(operator_id, config.span)
                    .flat_map(|task| task.usages())
                    .filter(|usage| resource_ids.contains(&usage.resource_id())),
            )?;
            self.timeline_to_ui(builder.build(), epoch)
        }
    }

    pub fn request_timelines_bulk(
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

        // Prepare builders, resource id filters, and operator filters, one for
        // each bulk entry. After populating this, we'll build a reverse index,
        // that maps a resource_id to a list of indices in these vecs, for which
        // that resource's usages are relevant.
        let mut plain_builders: Vec<(Uuid, ResourceTimelineBuilder)> = Vec::new();
        let mut plain_id_filters: Vec<HashSet<Uuid>> = Vec::new();
        let mut plain_op_filters: Vec<Option<Uuid>> = Vec::new();

        // Prepare them also for keyed builders (building by state).
        let mut per_state_builders: Vec<(Uuid, ResourceTimelineByKeyBuilder<&str>)> = Vec::new();
        let mut per_state_id_filters: Vec<HashSet<Uuid>> = Vec::new();
        let mut per_state_op_filters: Vec<Option<Uuid>> = Vec::new();

        // Prepare resource tree, we'll re-use this as it is potentially
        // expensive to build for every entry.
        let resource_tree = self.resource_tree(query_id)?;

        for (resource_or_group_id, params) in &request.entries {
            let BulkEntryBuilder {
                resource_type,
                id_filter,
                per_state,
                operator_id,
                long_entities_threshold,
            } = self.try_prepare_bulk_entry(*resource_or_group_id, &resource_tree, params)?;
            if per_state {
                per_state_op_filters.push(operator_id);
                per_state_id_filters.push(id_filter.clone());
                per_state_builders.push((
                    *resource_or_group_id,
                    ResourceTimelineByKeyBuilder::try_new(
                        resource_type,
                        config,
                        long_entities_threshold,
                    )?,
                ));
            } else {
                plain_op_filters.push(operator_id);
                plain_id_filters.push(id_filter.clone());
                plain_builders.push((
                    *resource_or_group_id,
                    ResourceTimelineBuilder::try_new(
                        resource_type,
                        config,
                        long_entities_threshold,
                    )?,
                ));
            }
        }

        // Build reverse index so given a resource_(group)_id we can quickly
        // look up all builders associated with a request into which we can push
        // a usage.
        //
        // This is more efficient than going over all usages for each builder,
        // since the number of usages is typically going to be MUCH larger than
        // the number of builders.
        let plain_index: HashMap<Uuid, Vec<usize>> = plain_id_filters
            .iter()
            .enumerate()
            .flat_map(|(i, id_filter)| id_filter.iter().map(move |&id| (id, i)))
            .fold(HashMap::default(), |mut acc, (id, i)| {
                acc.entry(id).or_default().push(i);
                acc
            });
        let per_state_index: HashMap<Uuid, Vec<usize>> = per_state_id_filters
            .iter()
            .enumerate()
            .flat_map(|(i, id_filter)| id_filter.iter().map(move |&id| (id, i)))
            .fold(HashMap::default(), |mut acc, (id, i)| {
                acc.entry(id).or_default().push(i);
                acc
            });

        // Iterate over all usages once and push any usages of resources in our
        // lookup table to their respective builders. For now we only have
        // tasks.
        for task in self.model.tasks.values() {
            let task_operator_id = task.operator_id();

            for usage in task.usages() {
                let resource_id = usage.resource_id();
                if let Some(indices) = plain_index.get(&resource_id) {
                    for &idx in indices {
                        if plain_op_filters[idx].is_none_or(|op| task_operator_id == Some(op)) {
                            plain_builders[idx].1.try_push(&usage)?;
                        }
                    }
                }
            }

            for (state_name, usage) in task.usages_with_state_names() {
                let resource_id = usage.resource_id();
                if let Some(indices) = per_state_index.get(&resource_id) {
                    for &idx in indices {
                        if per_state_op_filters[idx].is_none_or(|op| task_operator_id == Some(op)) {
                            per_state_builders[idx].1.try_push(state_name, &usage)?;
                        }
                    }
                }
            }
        }

        // Collect results for all requests.
        let mut result = HashMap::default();
        for (key, builder) in plain_builders {
            result.insert(
                key,
                BulkTimelineResponseEntry::Ok {
                    message: String::new(),
                    data: self.timeline_to_ui_bulk(builder.build(), epoch)?,
                },
            );
        }
        for (key, builder) in per_state_builders {
            result.insert(
                key,
                BulkTimelineResponseEntry::Ok {
                    message: String::new(),
                    data: self.timeline_to_ui_keyed_bulk(builder.build(), epoch)?,
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

    /// Given a resource_(group)_id and other request parameters, figure out what are:
    /// - The resource_type
    /// - For groups, the set of resources to aggregate for.
    /// - Whether this is a request to split out usage per state.
    /// - What operator ID filter to apply.
    /// - What the threshold is for long entities.
    fn try_prepare_bulk_entry<'a>(
        &'a self,
        resource_id: Uuid,
        tree: &ResourceTreeNode,
        params: &ui::timeline::BulkTimelineRequestParams,
    ) -> AnalyzerResult<BulkEntryBuilder<'a>> {
        let (
            resource_type,
            resource_id_filter,
            fsm_type_name,
            operator_id,
            long_entities_threshold,
        ) = match params {
            BulkTimelineRequestParams::Resource(p) => {
                let resource_type = self.model.resource_type_of(resource_id)?;
                (
                    resource_type,
                    [resource_id].into_iter().collect(),
                    p.fsm_type_name.as_deref(),
                    p.operator_id,
                    p.long_entities_threshold_s.map(to_nanosecs),
                )
            }
            BulkTimelineRequestParams::ResourceGroup(p) => {
                let resource_type = self.model.resource_type(&p.resource_type_name)?;
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
                    p.long_entities_threshold_s.map(to_nanosecs),
                )
            }
        };

        // TODO(johanpel): once this gets more FSMs this needs to be moved back
        // to the caller function
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
            long_entities_threshold,
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
    ) -> AnalyzerResult<ui::timeline::TimelineResponse> {
        let config_secs = result.config.try_to_secs_relative(epoch)?;
        let capacities_values = result
            .data
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        let long_fsms = self.task_entities_to_ui_fsm(&result.long_entities, epoch)?;
        Ok(ui::timeline::TimelineResponse::Binned(
            ui::timeline::ResourceTimelineBinned {
                config: config_secs,
                capacities_values,
                long_fsms,
            },
        ))
    }

    /// Convert a keyed timeline to a UI-compatible one.
    fn timeline_to_ui_keyed(
        &self,
        result: ResourceTimelineByKey<&str>,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<ui::timeline::TimelineResponse> {
        let config_secs = result.config.try_to_secs_relative(epoch)?;
        let mut capacities_states_values = StdHashMap::new();
        for ((state_name, capacity_name), values) in result.data {
            capacities_states_values
                .entry(capacity_name.to_owned())
                .or_insert_with(StdHashMap::new)
                .insert(state_name.to_owned(), values);
        }
        let long_fsms = self.task_entities_to_ui_fsm(&result.long_entities, epoch)?;
        Ok(ui::timeline::TimelineResponse::BinnedByState(
            ui::timeline::ResourceTimelineBinnedByState {
                config: config_secs,
                capacities_states_values,
                long_fsms,
            },
        ))
    }

    /// Convert a timeline to a UI-compatible bulk entry response.
    fn timeline_to_ui_bulk(
        &self,
        result: ResourceTimeline,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<BulkTimelineData> {
        let capacities_values = result
            .data
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        let long_fsms = self.task_entities_to_ui_fsm(&result.long_entities, epoch)?;
        Ok(BulkTimelineData::Binned {
            capacities_values,
            long_fsms,
        })
    }

    /// Convert a keyed timeline to a UI-compatible bulk entry response.
    fn timeline_to_ui_keyed_bulk(
        &self,
        result: ResourceTimelineByKey<&str>,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<BulkTimelineData> {
        let mut capacities_states_values = std::collections::HashMap::default();
        for ((state_name, capacity_name), values) in result.data {
            capacities_states_values
                .entry(capacity_name.to_owned())
                .or_insert_with(std::collections::HashMap::new)
                .insert(state_name.to_owned(), values);
        }
        let long_fsms = self.task_entities_to_ui_fsm(&result.long_entities, epoch)?;
        Ok(BulkTimelineData::BinnedByState {
            capacities_states_values,
            long_fsms,
        })
    }
}

/// Helper struct to build bulk timeline responses.
struct BulkEntryBuilder<'a> {
    resource_type: &'a ResourceTypeDecl,
    id_filter: HashSet<Uuid>,
    per_state: bool,
    operator_id: Option<Uuid>,
    long_entities_threshold: Option<TimeNanoSec>,
}
