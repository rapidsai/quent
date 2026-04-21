// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Collections of resources and resource groups

use std::collections::{HashSet, hash_map::Entry};

use rustc_hash::FxHashMap as HashMap;

use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult,
    resource::{
        CapacityDecl, Resource, ResourceGroup, ResourceGroupTypeDecl, ResourceTypeDecl,
        runtime::{RtResource, RtResourceBuilder, RtResourceGroup},
        tree::ResourceTreeNode,
    },
};

/// Trait for types holding a collection of [`Resource`]s and/or [`ResourceGroup`]s.
pub trait ResourceCollection {
    /// Return an iterator over all `Resource`s in this collection.
    fn resources(&self) -> impl Iterator<Item = &dyn Resource>;

    /// Return an iterator over all `ResourceGroup`s in this collection.
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

/// A builder for `InMemoryResourceCollection`.
#[derive(Default)]
pub struct InMemoryResourcesBuilder {
    resource_types: HashMap<String, ResourceTypeDecl>,
    resources: HashMap<Uuid, RtResourceBuilder>,
    resource_groups: HashMap<Uuid, RtResourceGroup>,
}

/// An in-memory collection of [`Resource`]s and [`ResourceGroup`]s.
pub struct InMemoryResources {
    pub resource_types: HashMap<String, ResourceTypeDecl>,
    pub resources: HashMap<Uuid, RtResource>,
    pub resource_groups: HashMap<Uuid, RtResourceGroup>,
}

impl InMemoryResourcesBuilder {
    /// Get or create a resource builder for the given ID.
    pub fn try_builder(&mut self, id: Uuid) -> AnalyzerResult<&mut RtResourceBuilder> {
        if let Entry::Vacant(e) = self.resources.entry(id) {
            e.insert(RtResourceBuilder::try_new(id)?);
        }
        Ok(self.resources.get_mut(&id).unwrap())
    }

    /// Register a memory resource type declaration (bytes occupancy capacity).
    pub fn insert_memory_resource(&mut self, type_name: &str) {
        if !self.resource_types.contains_key(type_name) {
            let decl =
                ResourceTypeDecl::new(type_name, [CapacityDecl::new_occupancy("capacity_bytes")]);
            self.resource_types.insert(type_name.to_owned(), decl);
        }
    }

    /// Register a processor (unit) resource type declaration.
    pub fn insert_processor_resource(&mut self, type_name: &str) {
        if !self.resource_types.contains_key(type_name) {
            let decl = ResourceTypeDecl::unit(type_name);
            self.resource_types.insert(type_name.to_owned(), decl);
        }
    }

    /// Register a channel resource type declaration (bytes rate capacity).
    // TODO(johanpel): see CapacityType and consider blocking/non-blocking channels
    pub fn insert_channel_resource(&mut self, type_name: &str) {
        if !self.resource_types.contains_key(type_name) {
            let decl = ResourceTypeDecl::new(type_name, [CapacityDecl::new_rate("capacity_bytes")]);
            self.resource_types.insert(type_name.to_owned(), decl);
        }
    }

    /// Insert a resource group directly from individual fields.
    pub fn push_group_raw(
        &mut self,
        id: Uuid,
        type_name: &str,
        instance_name: &str,
        parent_group_id: Option<Uuid>,
    ) {
        let _ = self.resource_groups.insert(
            id,
            RtResourceGroup {
                id,
                type_name: type_name.to_owned(),
                instance_name: instance_name.to_owned(),
                parent_group_id,
            },
        );
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
