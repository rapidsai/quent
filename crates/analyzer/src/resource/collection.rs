//! Collections of resources and resource groups

use std::collections::{HashSet, hash_map::Entry};

use rustc_hash::FxHashMap as HashMap;

use quent_events::{
    Event,
    resource::{
        ResourceEvent, channel::ChannelEvent, memory::MemoryEvent, processor::ProcessorEvent,
    },
};

use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult,
    resource::{
        CapacityDecl, CapacityValue, Resource, ResourceCapacities, ResourceGroup,
        ResourceGroupTypeDecl, ResourceTypeDecl,
        runtime::{RtResource, RtResourceBuilder, RtResourceGroup, RtResourceStateTransition},
        tree::ResourceTreeNode,
    },
};

/// Trait for types holding a collection of [`Resource`]s and/or [`ResourceGroup`]s.
pub trait ResourceCollection {
    /// Return an iterator over all [`Resources`] in this collection.
    fn resources(&self) -> impl Iterator<Item = &dyn Resource>;

    /// Return an iterator over all [`ResourceGroups`] in this collection.
    fn resource_groups(&self) -> impl Iterator<Item = &dyn ResourceGroup>;

    /// Return a reference to the [`Resource`] with the provided ID.
    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource>;

    /// Return the [`ResourceTypeDecl`] of the resource type with the provided
    /// resource type name.
    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl>;

    /// Return the [`ResourceTypeDecl`] of the resource type with the provided
    /// resource .
    fn resource_type_of(&self, resource_id: Uuid) -> AnalyzerResult<&ResourceTypeDecl> {
        self.resource_type(self.resource(resource_id)?.type_name())
    }

    /// Return the [`ResourceGroup`] of the resource group with the provided ID.
    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup>;

    /// Return references to directly nested [`ResourceGroup`] children of the resource group with the provided id.
    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>>;

    /// Return references to directly nested [`Resource`] children of the resource group with the provided id.
    fn resource_group_child_resources(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>>;
}

/// Convenience function to derive and populate all [`ResourceGroupTypeDecl`]s
/// by walking the resource tree of a resource collection.
pub fn derive_resource_group_types(
    collection: &impl ResourceCollection,
) -> AnalyzerResult<HashMap<String, ResourceGroupTypeDecl>> {
    let mut resource_group_types: HashMap<String, ResourceGroupTypeDecl> = HashMap::default();

    // Find all root groups (those with no parent)
    let root_groups: Vec<Uuid> = collection
        .resource_groups()
        .filter_map(|group| group.parent_group_id().is_none().then_some(group.id()))
        .collect();

    // Walk each root group tree
    for root_group_id in root_groups {
        let tree = ResourceTreeNode::try_new(collection, root_group_id)?;
        populate_group_types_from_tree(&tree, collection, &mut resource_group_types)?;
    }

    Ok(resource_group_types)
}

fn populate_group_types_from_tree(
    node: &crate::resource::tree::ResourceTreeNode,
    collection: &impl ResourceCollection,
    group_types: &mut HashMap<String, ResourceGroupTypeDecl>,
) -> AnalyzerResult<()> {
    match node {
        ResourceTreeNode::ResourceGroup(group_id, children) => {
            let group = collection.resource_group(*group_id)?;
            let group_type = group.type_name();

            // Insert type if not present
            if !group_types.contains_key(group_type) {
                group_types.insert(
                    group_type.to_owned(),
                    group.resource_group_type_decl(HashSet::new(), HashSet::new()),
                );
            }

            // Collect all resource types in this subtree
            for resource_id in node.iter_leaf_ids() {
                let resource = collection.resource(resource_id)?;
                let resource_type = resource.type_name();

                if let Some(group_type_decl) = group_types.get_mut(group_type) {
                    group_type_decl
                        .contains_resource_types
                        .insert(resource_type.to_owned());
                }
            }

            for child in children {
                populate_group_types_from_tree(child, collection, group_types)?;
            }
        }
        ResourceTreeNode::Resource(_) => {}
    }
    Ok(())
}

/// A builder for [`InMemoryResourceCollection`].
#[derive(Default)]
pub struct InMemoryResourcesBuilder {
    resource_types: HashMap<String, ResourceTypeDecl>,
    resources: HashMap<Uuid, RtResourceBuilder>,
    resource_groups: HashMap<Uuid, RtResourceGroup>,
}

/// An in-memory collection of [`Resource`]s and [`ResourceGroup`]s.
#[derive(Debug)]
pub struct InMemoryResources {
    pub resource_types: HashMap<String, ResourceTypeDecl>,
    pub resources: HashMap<Uuid, RtResource>,
    pub resource_groups: HashMap<Uuid, RtResourceGroup>,
}

impl InMemoryResourcesBuilder {
    fn try_builder(&mut self, id: Uuid) -> AnalyzerResult<&mut RtResourceBuilder> {
        if let Entry::Vacant(e) = self.resources.entry(id) {
            e.insert(RtResourceBuilder::try_new(id)?);
        }
        Ok(self.resources.get_mut(&id).unwrap())
    }

    fn insert_memory_resource(&mut self, type_name: &str) {
        if !self.resource_types.contains_key(type_name) {
            let decl = ResourceTypeDecl::new(type_name, [CapacityDecl::new_occupancy("bytes")]);
            self.resource_types.insert(type_name.to_owned(), decl);
        }
    }
    fn insert_processor_resource(&mut self, type_name: &str) {
        if !self.resource_types.contains_key(type_name) {
            let decl = ResourceTypeDecl::unit(type_name);
            self.resource_types.insert(type_name.to_owned(), decl);
        }
    }
    // TODO(johanpel): see CapacityType and consider blocking/non-blocking channels
    fn insert_channel_resource(&mut self, type_name: &str) {
        if !self.resource_types.contains_key(type_name) {
            let decl = ResourceTypeDecl::new(type_name, [CapacityDecl::new_rate("bytes")]);
            self.resource_types.insert(type_name.to_owned(), decl);
        }
    }

    pub fn try_push(&mut self, event: Event<ResourceEvent>) -> AnalyzerResult<()> {
        let Event {
            id,
            timestamp,
            data,
        } = event;

        match data {
            ResourceEvent::Memory(memory_event) => match memory_event {
                MemoryEvent::Init(init) => {
                    self.insert_memory_resource(&init.resource.type_name);
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Init(timestamp));
                    bld.set_type_name(init.resource.type_name);
                    bld.set_instance_name(Some(init.resource.instance_name));
                    bld.set_parent_group_id(init.resource.parent_group_id);
                }
                MemoryEvent::Operating(operating) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Operating(
                        timestamp,
                        ResourceCapacities(vec![CapacityValue::new(
                            "bytes",
                            operating.capacity_bytes,
                        )]),
                    ));
                }
                MemoryEvent::Resizing(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Resizing(timestamp));
                }
                MemoryEvent::Finalizing(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Finalizing(timestamp));
                }
                MemoryEvent::Exit(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Exit(timestamp));
                }
            },
            ResourceEvent::Processor(processor_resource_event) => match processor_resource_event {
                ProcessorEvent::Init(init) => {
                    self.insert_processor_resource(&init.resource.type_name);
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Init(timestamp));
                    bld.set_type_name(init.resource.type_name);
                    bld.set_instance_name(Some(init.resource.instance_name));
                    bld.set_parent_group_id(init.resource.parent_group_id);
                }
                ProcessorEvent::Operating(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Operating(
                        timestamp,
                        ResourceCapacities(vec![]),
                    ));
                }
                ProcessorEvent::Finalizing(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Finalizing(timestamp));
                }
                ProcessorEvent::Exit(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Exit(timestamp));
                }
            },
            ResourceEvent::Channel(channel_resource_event) => match channel_resource_event {
                ChannelEvent::Init(init) => {
                    self.insert_channel_resource(&init.resource.type_name);
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Init(timestamp));
                    bld.set_type_name(init.resource.type_name);
                    bld.set_instance_name(Some(init.resource.instance_name));
                    bld.set_parent_group_id(init.resource.parent_group_id);
                }
                ChannelEvent::Operating(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Operating(
                        timestamp,
                        ResourceCapacities(vec![]),
                    ));
                }
                ChannelEvent::Finalizing(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Finalizing(timestamp));
                }
                ChannelEvent::Exit(_) => {
                    let bld = self.try_builder(id)?;
                    bld.push(RtResourceStateTransition::Exit(timestamp));
                }
            },
            ResourceEvent::Group(group_event) => {
                self.resource_groups.insert(
                    id,
                    RtResourceGroup::try_new(
                        id,
                        group_event.type_name,
                        group_event.instance_name,
                        group_event.parent_group_id,
                    )?,
                );
            }
        }

        Ok(())
    }

    pub fn try_extend(
        &mut self,
        iterator: impl Iterator<Item = Event<ResourceEvent>>,
    ) -> AnalyzerResult<()> {
        for event in iterator {
            self.try_push(event)?;
        }
        Ok(())
    }

    pub fn try_build(self) -> AnalyzerResult<InMemoryResources> {
        let resources: HashMap<Uuid, RtResource> = self
            .resources
            .into_iter()
            .map(|(id, builder)| builder.try_build().map(|resource| (id, resource)))
            .collect::<AnalyzerResult<_>>()?;

        Ok(InMemoryResources {
            resource_types: self.resource_types,
            resources,
            resource_groups: self.resource_groups,
        })
    }
}

impl ResourceCollection for InMemoryResources {
    fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
        self.resources.values().map(|r| r as &dyn Resource)
    }
    fn resource_groups(&self) -> impl Iterator<Item = &dyn ResourceGroup> {
        self.resource_groups
            .values()
            .map(|g| g as &dyn ResourceGroup)
    }
    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
        self.resources
            .get(&resource_id)
            .map(|r| r as &dyn Resource)
            .ok_or(AnalyzerError::InvalidId(resource_id))
    }
    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
        self.resource_types
            .get(resource_type_name)
            .ok_or(AnalyzerError::InvalidTypeName(format!(
                "unknown resource type {resource_type_name}"
            )))
    }
    fn resource_group(&self, resource_group_id: Uuid) -> AnalyzerResult<&dyn ResourceGroup> {
        self.resource_groups
            .get(&resource_group_id)
            .map(|g| g as &dyn ResourceGroup)
            .ok_or(AnalyzerError::InvalidId(resource_group_id))
    }
    fn resource_group_child_groups(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        // sanity check
        self.resource_group(resource_group_id)?;
        Ok(self.resource_groups.values().filter_map(move |group| {
            group
                .parent_group_id
                .and_then(|parent| (parent == resource_group_id).then_some(group.id))
        }))
    }
    fn resource_group_child_resources(
        &self,
        resource_group_id: Uuid,
    ) -> AnalyzerResult<impl Iterator<Item = Uuid>> {
        self.resource_group(resource_group_id)?;
        Ok(self.resources.values().filter_map(move |resource| {
            (resource.parent_group_id == resource_group_id).then_some(resource.id)
        }))
    }
}

#[cfg(test)]
impl InMemoryResources {
    pub(crate) fn insert_resource(&mut self, res: RtResource) {
        self.resources.insert(res.id, res);
    }
    pub(crate) fn insert_type(&mut self, typ: ResourceTypeDecl) {
        self.resource_types.insert(typ.name.clone(), typ);
    }
}
