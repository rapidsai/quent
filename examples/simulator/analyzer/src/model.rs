use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::collection::FsmCollection,
    resource::{
        Resource, ResourceGroup, ResourceGroupTypeDecl, ResourceTypeDecl, Usage, Using,
        collection::{
            InMemoryResources, InMemoryResourcesBuilder, ResourceCollection,
            derive_resource_group_types,
        },
    },
};
use quent_events::Event;
use quent_query_engine_analyzer::{
    InMemoryEngineModel, InMemoryEngineModelBuilder, InMemoryEngineModelQueryView,
};
use quent_simulator_events::SimulatorEvent;
use quent_simulator_ui::EntityRef;
use quent_time::span::SpanUnixNanoSec;
use uuid::Uuid;

use quent_query_engine_analyzer as qea;

use crate::{
    convert_ref,
    task::{Task, TaskBuilder},
};

pub struct SimulatorModelBuilder {
    query_engine: InMemoryEngineModelBuilder,
    resources: InMemoryResourcesBuilder,
    tasks: HashMap<Uuid, TaskBuilder>,
}

/// A model of the simulator engine
#[derive(Debug)]
pub struct SimulatorModel {
    pub query_engine: InMemoryEngineModel,
    pub resources: InMemoryResources,
    pub tasks: Vec<Task>,
    pub resource_group_types: HashMap<String, ResourceGroupTypeDecl>,
}

/// A view of the simulator model filtered to a specific query
// TODO(johanpel): figure out a better way to construct these views, or to
// filter the data on a per query basis. This is generally tricky because the
// state of resources of engines that are shared across query groups or across
// the entire engine could be modified by other queries.
pub struct SimulatorModelQueryView<'a> {
    pub query_engine_view: InMemoryEngineModelQueryView<'a>,
    pub resources: &'a InMemoryResources,
    pub tasks: HashMap<Uuid, &'a Task>,
}

impl<'a> SimulatorModelQueryView<'a> {
    pub fn try_new(
        model: &'a SimulatorModel,
        query_id: Uuid,
    ) -> AnalyzerResult<SimulatorModelQueryView<'a>> {
        let query_engine_view =
            InMemoryEngineModelQueryView::try_new(&model.query_engine, query_id)?;

        let mut result = SimulatorModelQueryView {
            query_engine_view,
            resources: &model.resources,
            tasks: HashMap::default(),
        };

        result.tasks = model
            .tasks
            .iter()
            .map(|task| (task.id(), task))
            .filter(|(_, task)| {
                task.usages()
                    .any(|(usage, _)| result.resource(usage.resource).is_ok())
            })
            .collect();
        Ok(result)
    }

    pub fn entity_ref(&self, id: Uuid) -> Option<qea::EntityRef> {
        // Check query engine entities first
        self.query_engine_view.entity_ref(id)
    }
}

impl<'a> ResourceCollection for SimulatorModelQueryView<'a> {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        self.resources.resources()
    }

    fn resource_groups(&self) -> impl Iterator<Item = &dyn ResourceGroup> {
        // Query engine entities
        let engine = std::iter::once(self.query_engine_view.engine as &dyn ResourceGroup);
        let workers = self
            .query_engine_view
            .workers
            .values()
            .map(|&w| w as &dyn ResourceGroup);
        let query_groups =
            std::iter::once(self.query_engine_view.query_group as &dyn ResourceGroup);
        let queries = std::iter::once(self.query_engine_view.query as &dyn ResourceGroup);
        let plans = self
            .query_engine_view
            .plans
            .values()
            .map(|&p| p as &dyn ResourceGroup);
        let operators = self
            .query_engine_view
            .operators
            .values()
            .map(|&o| o as &dyn ResourceGroup);
        let ports = self
            .query_engine_view
            .ports
            .values()
            .map(|&p| p as &dyn ResourceGroup);

        // Simulator resource groups
        let sim_groups = self.resources.resource_groups();

        engine
            .chain(workers)
            .chain(query_groups)
            .chain(queries)
            .chain(plans)
            .chain(operators)
            .chain(ports)
            .chain(sim_groups)
    }

    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
        self.resources.resource(resource_id)
    }

    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
        self.resources.resource_type(resource_type_name)
    }

    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup> {
        // Check query engine entities first
        if self.query_engine_view.engine.id == resource_group_id {
            return Ok(self.query_engine_view.engine);
        }
        if let Some(&worker) = self.query_engine_view.workers.get(&resource_group_id) {
            return Ok(worker);
        }
        if self.query_engine_view.query_group.id == resource_group_id {
            return Ok(self.query_engine_view.query_group);
        }
        if self.query_engine_view.query.id == resource_group_id {
            return Ok(self.query_engine_view.query);
        }
        if let Some(&plan) = self.query_engine_view.plans.get(&resource_group_id) {
            return Ok(plan);
        }
        if let Some(&operator) = self.query_engine_view.operators.get(&resource_group_id) {
            return Ok(operator);
        }
        if let Some(&port) = self.query_engine_view.ports.get(&resource_group_id) {
            return Ok(port);
        }

        // Fall back to simulator resources
        self.resources.resource_group(resource_group_id)
    }

    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Verify the resource group exists
        self.resource_group(resource_group_id)?;

        // Get children from query engine entities that are in this query
        let engine_children = if self.query_engine_view.engine.id == resource_group_id {
            self.query_engine_view
                .workers
                .keys()
                .copied()
                .collect::<Vec<_>>()
        } else if self
            .query_engine_view
            .workers
            .contains_key(&resource_group_id)
        {
            self.query_engine_view
                .plans
                .values()
                .filter(|p| p.worker_id == Some(resource_group_id))
                .map(|p| p.id)
                .collect()
        } else if self.query_engine_view.query_group.id == resource_group_id {
            vec![self.query_engine_view.query.id]
        } else if self.query_engine_view.query.id == resource_group_id {
            // All plans in the view belong to this query
            self.query_engine_view.plans.keys().copied().collect()
        } else if let Some(plan) = self.query_engine_view.plans.get(&resource_group_id) {
            self.query_engine_view
                .operators
                .values()
                .filter(|o| o.plan_id == Some(plan.id))
                .map(|o| o.id)
                .collect()
        } else if let Some(operator) = self.query_engine_view.operators.get(&resource_group_id) {
            self.query_engine_view
                .ports
                .values()
                .filter(|p| p.operator_id == Some(operator.id))
                .map(|p| p.id)
                .collect()
        } else {
            vec![]
        };

        // Get children from simulator resources
        let sim_children = self
            .resources
            .resource_groups
            .values()
            .filter(move |group| group.parent_group_id == Some(resource_group_id))
            .map(|group| group.id);

        Ok(engine_children.into_iter().chain(sim_children))
    }

    fn resource_group_child_resources(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Verify the resource group exists
        self.resource_group(resource_group_id)?;

        // Get child resources from simulator resources only
        let children = self
            .resources
            .resources
            .values()
            .filter(move |resource| resource.parent_group_id() == resource_group_id)
            .map(|resource| resource.id);

        Ok(children)
    }
}

impl SimulatorModelBuilder {
    pub fn try_new(engine_id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self {
            query_engine: InMemoryEngineModelBuilder::try_new(engine_id)?,
            resources: InMemoryResourcesBuilder::default(),
            tasks: HashMap::default(),
        })
    }

    pub fn try_push(&mut self, event: Event<SimulatorEvent>) -> AnalyzerResult<()> {
        let Event {
            id,
            timestamp,
            data,
        } = event;
        match data {
            SimulatorEvent::Task(t) => {
                let task_builder = self
                    .tasks
                    .entry(event.id)
                    .or_insert_with(|| TaskBuilder::try_new(event.id).unwrap());
                task_builder.push(Event::new(id, timestamp, t));
                Ok(())
            }
            SimulatorEvent::QueryEngineEvent(qe) => {
                self.query_engine.try_push(Event::new(id, timestamp, qe))
            }
            SimulatorEvent::Resource(r) => self.resources.try_push(Event::new(id, timestamp, r)),
        }
    }

    pub fn try_build(self) -> AnalyzerResult<SimulatorModel> {
        // Build resources first. As we iterate over task builders and build all
        // tasks, we can populate the leaf resources used_by field.
        let mut resources = self.resources.try_build()?;
        let query_engine = self.query_engine.try_build()?;

        let mut tasks = Vec::with_capacity(self.tasks.len());

        for (_task_id, task_builder) in self.tasks.into_iter() {
            let task = task_builder.try_build()?;
            for (usage, _) in task.usages() {
                let resource_type_name = resources.resource(usage.resource)?.type_name().to_owned();
                let set = &mut resources
                    .resource_types
                    .get_mut(&resource_type_name)
                    .unwrap()
                    .used_by;
                if !set.contains(task.type_name()) {
                    set.insert(task.type_name().to_owned());
                }
            }
            tasks.push(task);
        }

        // Construct the model without group type decls being populated yet, we
        // will populate it based on the resource tree.
        let temp_model = SimulatorModel {
            query_engine,
            resources,
            tasks,
            resource_group_types: HashMap::default(),
        };
        let mut resource_group_types = derive_resource_group_types(&temp_model)?;
        // Bubble up all the used_by_entity fields in the group type decls.
        for group_type_decl in resource_group_types.values_mut() {
            for contained_resource_type in &group_type_decl.contains_resource_types {
                if let Ok(resource_type) =
                    temp_model.resources.resource_type(contained_resource_type)
                {
                    for entity_type in &resource_type.used_by {
                        group_type_decl
                            .used_by_entity_types
                            .insert(entity_type.clone());
                    }
                }
            }
        }

        Ok(SimulatorModel {
            query_engine: temp_model.query_engine,
            resources: temp_model.resources,
            tasks: temp_model.tasks,
            resource_group_types,
        })
    }
}

impl FsmCollection<Task> for SimulatorModel {
    fn fsms<'a>(&'a self) -> impl Iterator<Item = &'a Task> + 'a
    where
        Task: 'a,
    {
        self.tasks.iter()
    }

    fn contains_fsm_type(&self, type_name: &str) -> bool {
        !self.tasks.is_empty() && type_name == "task"
    }
}

impl ResourceCollection for SimulatorModel {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        self.resources
            .resources()
            .chain(self.query_engine.resources())
    }
    fn resource_groups(&self) -> impl Iterator<Item = &dyn ResourceGroup> {
        self.resources
            .resource_groups()
            .chain(self.query_engine.resource_groups())
    }
    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
        self.resources
            .resource(resource_id)
            .or_else(|_| self.query_engine.resource(resource_id))
    }
    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
        self.query_engine
            .resource_type(resource_type_name)
            .or_else(|_| self.resources.resource_type(resource_type_name))
    }
    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup> {
        self.query_engine
            .resource_group(resource_group_id)
            .or_else(|_| self.resources.resource_group(resource_group_id))
    }

    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Verify the resource group exists in at least one collection
        self.resource_group(resource_group_id)?;

        let engine = self
            .query_engine
            .resource_group_child_groups(resource_group_id)
            .ok();

        let sim = self
            .resources
            .resource_groups
            .values()
            .filter_map(move |group| {
                group
                    .parent_group_id
                    .and_then(|parent| (parent == resource_group_id).then_some(group.id))
            });

        Ok(engine.into_iter().flatten().chain(sim))
    }

    fn resource_group_child_resources(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // Verify the resource group exists in at least one collection
        self.resource_group(resource_group_id)?;

        let engine = self
            .query_engine
            .resource_group_child_resources(resource_group_id)
            .ok();

        let sim = self
            .resources
            .resources
            .values()
            .filter_map(move |resource| {
                (resource.parent_group_id() == resource_group_id).then_some(resource.id)
            });

        Ok(engine.into_iter().flatten().chain(sim))
    }
}

impl SimulatorModel {
    pub fn entity_ref(&self, id: Uuid) -> AnalyzerResult<EntityRef> {
        if let Some(r) = self.query_engine.entity_ref(id) {
            Ok(convert_ref(r))
        } else if self.resources.resource(id).is_ok() {
            Ok(EntityRef::Resource(id))
        } else if self.resources.resource_group(id).is_ok() {
            Ok(EntityRef::ResourceGroup(id))
        } else {
            Err(AnalyzerError::InvalidId(id))
        }
    }
}

impl Using for SimulatorModel {
    fn usages(&self) -> impl Iterator<Item = (&Usage, SpanUnixNanoSec)> {
        self.tasks.iter().flat_map(|task| task.usages())
    }
}
