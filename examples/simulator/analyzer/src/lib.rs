use std::collections::HashSet;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity, Span,
    resource::{collection::ResourceCollection, tree::ResourceTreeNode},
    timeline::binned::resource::{ResourceTimelineBinned, ResourceTimelineBinnedByState},
};
use quent_events::Event;
use quent_query_engine_analyzer::{self as qea};
use quent_simulator_events::SimulatorEvent;
use quent_simulator_ui::{
    self as ui, EntityRef, QueryBundle, QueryEntities, ResourceGroupNode, ResourceTree,
};
use quent_time::{TimeUnixNanoSec, bin::BinnedSpan, to_secs};
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
) -> AnalyzerResult<Option<ResourceTree>> {
    match node {
        ResourceTreeNode::ResourceGroup(id, children) => {
            let entity_ref = view
                .entity_ref(id)
                .map(convert_ref)
                .unwrap_or(EntityRef::ResourceGroup(id));
            let children: Vec<ResourceTree> = children
                .into_iter()
                .map(|child| convert_resource_tree(child, view))
                .collect::<AnalyzerResult<Vec<Option<ResourceTree>>>>()?
                .into_iter()
                .flatten()
                .collect();
            if !children.is_empty() {
                Ok(Some(ResourceTree::ResourceGroup(ResourceGroupNode {
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
            Ok(Some(ResourceTree::Resource(entity_ref)))
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
        for event in events {
            builder.try_push(event)?;
        }
        Ok(Self {
            model: builder.try_build()?,
        })
    }

    pub fn resource_tree(&self, query_id: Uuid) -> AnalyzerResult<ResourceTree> {
        let view = SimulatorModelQueryView::try_new(&self.model, query_id)?;
        let tree_node = ResourceTreeNode::try_new(&view, view.query_engine_view.engine.id)?;
        convert_resource_tree(tree_node, &view)?.ok_or_else(|| {
            AnalyzerError::Validation(format!("query {query_id} has no top-level resource group"))
        })
    }

    pub fn query_bundle(&self, query_id: Uuid) -> AnalyzerResult<QueryBundle> {
        let qe = &self.model.query_engine;

        let start_time_unix_ns = qe.query_epoch(query_id)?;
        let duration_s = to_secs(qe.query(query_id)?.span()?.duration());

        let view = qe.query_view(query_id)?;

        let epoch = qe.query_epoch(query_id)?;

        Ok(QueryBundle {
            query_id,
            entities: QueryEntities {
                engine: view.engine.try_into()?,
                query_group: view.query_group.into(),
                query: view.query.try_into()?,
                workers: view
                    .workers
                    .values()
                    .map(|&w| (w, epoch).try_into().map(|v| (w.id(), v)))
                    .collect::<AnalyzerResult<_>>()?,
                plans: view
                    .plans
                    .values()
                    .map(|&p| (p, epoch).try_into().map(|v| (p.id(), v)))
                    .collect::<AnalyzerResult<_>>()?,
                operators: view
                    .operators
                    .values()
                    .map(|&o| (o, epoch).try_into().map(|v| (o.id(), v)))
                    .collect::<AnalyzerResult<_>>()?,
                ports: view
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
                resource_groups: self
                    .model
                    .resources
                    .resource_groups()
                    .map(|res| (res.id(), res.into()))
                    .collect(),
                resource_types: self
                    .model
                    .resources
                    .resource_types
                    .iter()
                    .map(|(k, v)| (k.clone(), v.into()))
                    .collect(),
            },
            plan_tree: (&self.model.query_engine.plan_tree(query_id)?).into(),
            resource_tree: self.resource_tree(query_id)?,
            unique_operator_names: view
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
        config: BinnedSpan,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<ui::TimelineResponse> {
        let config_secs = config.try_to_secs_relative(epoch)?;
        Ok(if let Some(fsm_type_name) = fsm_type_name {
            let timeline = ResourceTimelineBinnedByState::try_new_resource(
                &self.model,
                &self.model,
                resource_id,
                &fsm_type_name,
                config,
            )?;
            ui::TimelineResponse::BinnedByState(ui::ResourceTimelineBinnedByState {
                config: config_secs,
                capacities_states_values: timeline.capacities_states_values,
            })
        } else {
            let timeline = ResourceTimelineBinned::try_new_resource(
                &self.model,
                &self.model,
                resource_id,
                config,
            )?;
            ui::TimelineResponse::Binned(ui::ResourceTimelineBinned {
                config: config_secs,
                capacities_values: timeline.capacities_values,
            })
        })
    }

    pub fn resource_group_timeline(
        &self,
        resource_group_id: Uuid,
        resource_type_name: &str,
        fsm_type_name: Option<String>,
        config: BinnedSpan,
        epoch: TimeUnixNanoSec,
    ) -> AnalyzerResult<ui::TimelineResponse> {
        let config_secs = config.try_to_secs_relative(epoch)?;
        Ok(if let Some(fsm_type_name) = fsm_type_name {
            let timeline = ResourceTimelineBinnedByState::try_new_group(
                &self.model,
                &self.model,
                resource_group_id,
                resource_type_name,
                &fsm_type_name,
                config,
            )?;
            ui::TimelineResponse::BinnedByState(ui::ResourceTimelineBinnedByState {
                config: config_secs,
                capacities_states_values: timeline.capacities_states_values,
            })
        } else {
            let timeline = ResourceTimelineBinned::try_new_group(
                &self.model,
                &self.model,
                resource_group_id,
                resource_type_name,
                config,
            )?;
            ui::TimelineResponse::Binned(ui::ResourceTimelineBinned {
                config: config_secs,
                capacities_values: timeline.capacities_values,
            })
        })
    }
}
