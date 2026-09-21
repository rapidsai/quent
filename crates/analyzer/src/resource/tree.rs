// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resource hierarchy construction and traversal for schemas using both the
//! `quent-ref-tree` and `quent-resource` constraints.

use std::collections::VecDeque;

use rustc_hash::FxHashSet as HashSet;
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult,
    ref_tree::{RefTreeCollection, RefTreeNode},
    resource::{Resource, collection::ResourceCollection},
};

/// An entity in a resource hierarchy, optionally marked as a resource.
pub struct ResourceTreeNode {
    pub entity_id: Uuid,
    pub is_resource: bool,
    pub children: Vec<ResourceTreeNode>,
}

impl ResourceTreeNode {
    /// Construct a resource hierarchy from a validated Reference Tree.
    pub fn try_new(
        collection: &(impl RefTreeCollection + ResourceCollection),
    ) -> AnalyzerResult<Self> {
        Self::try_from_ref_tree(RefTreeNode::try_new(collection)?, collection)
    }

    /// Construct a resource hierarchy from a validated Reference Tree.
    ///
    /// Construction takes `O(e + r)` time and storage for `e` Reference Tree
    /// entities and `r` resources.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerError::Validation`] if resource IDs are duplicated or
    /// do not occur in the Reference Tree.
    pub fn try_from_ref_tree(
        ref_tree: RefTreeNode,
        resources: &impl ResourceCollection,
    ) -> AnalyzerResult<Self> {
        let resources = resources.resources();
        let mut resource_ids =
            HashSet::with_capacity_and_hasher(resources.size_hint().0, Default::default());
        for resource in resources {
            let resource_id = resource.id();
            if !resource_ids.insert(resource_id) {
                return Err(AnalyzerError::Validation(format!(
                    "duplicate resource id {resource_id}"
                )));
            }
        }

        let tree = Self::from_ref_tree(ref_tree, &mut resource_ids);
        if let Some(resource_id) = resource_ids.into_iter().next() {
            return Err(AnalyzerError::Validation(format!(
                "resource {resource_id} is absent from the Reference Tree"
            )));
        }
        Ok(tree)
    }

    fn from_ref_tree(ref_tree: RefTreeNode, resource_ids: &mut HashSet<Uuid>) -> Self {
        let RefTreeNode {
            entity_id,
            children,
        } = ref_tree;
        Self {
            entity_id,
            is_resource: resource_ids.remove(&entity_id),
            children: children
                .into_iter()
                .map(|child| Self::from_ref_tree(child, resource_ids))
                .collect(),
        }
    }

    /// Return the IDs of all resources in this hierarchy.
    pub fn iter_resource_ids(&self) -> ResourceTreeResourceIter<'_> {
        ResourceTreeResourceIter { stack: vec![self] }
    }

    /// Return references to all resources in this hierarchy.
    pub fn iter_resource_refs<'a>(
        &self,
        resources: &'a impl ResourceCollection,
    ) -> impl Iterator<Item = AnalyzerResult<&'a dyn Resource>> {
        self.iter_resource_ids().map(|id| resources.resource(id))
    }

    /// Breadth-first search for a specific entity ID.
    pub fn find(&self, target_id: Uuid) -> Option<&Self> {
        let mut queue = VecDeque::new();
        queue.push_back(self);

        while let Some(node) = queue.pop_front() {
            if node.entity_id == target_id {
                return Some(node);
            }
            queue.extend(node.children.iter());
        }

        None
    }
}

/// Iterator over all resources in a [`ResourceTreeNode`].
pub struct ResourceTreeResourceIter<'a> {
    stack: Vec<&'a ResourceTreeNode>,
}

impl Iterator for ResourceTreeResourceIter<'_> {
    type Item = Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            self.stack.extend(node.children.iter().rev());
            if node.is_resource {
                return Some(node.entity_id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::collection::test_support::TestResources;

    fn resources(resource_ids: impl IntoIterator<Item = Uuid>) -> TestResources {
        let mut resources = TestResources::default();
        for resource_id in resource_ids {
            resources.insert_resource(resource_id, "test");
        }
        resources
    }

    fn node(
        entity_id: Uuid,
        is_resource: bool,
        children: Vec<ResourceTreeNode>,
    ) -> ResourceTreeNode {
        ResourceTreeNode {
            entity_id,
            is_resource,
            children,
        }
    }

    #[test]
    fn iterates_resources() {
        let first_resource_id = Uuid::from_u128(1);
        let second_resource_id = Uuid::from_u128(2);
        let group_id = Uuid::from_u128(3);
        let root_id = Uuid::from_u128(4);
        let tree = node(
            root_id,
            false,
            vec![
                node(first_resource_id, true, vec![]),
                node(
                    group_id,
                    false,
                    vec![node(second_resource_id, true, vec![])],
                ),
            ],
        );

        assert_eq!(
            tree.iter_resource_ids().collect::<Vec<_>>(),
            [first_resource_id, second_resource_id]
        );
    }

    #[test]
    fn resource_may_have_resource_children() {
        let parent_resource_id = Uuid::from_u128(1);
        let child_resource_id = Uuid::from_u128(2);
        let tree = node(
            parent_resource_id,
            true,
            vec![node(child_resource_id, true, vec![])],
        );

        assert_eq!(
            tree.iter_resource_ids().collect::<Vec<_>>(),
            [parent_resource_id, child_resource_id]
        );
    }

    #[test]
    fn finds_entities() {
        let root_id = Uuid::from_u128(1);
        let child_id = Uuid::from_u128(2);
        let tree = node(root_id, false, vec![node(child_id, true, vec![])]);

        assert_eq!(tree.find(child_id).unwrap().entity_id, child_id);
        assert!(tree.find(Uuid::from_u128(3)).is_none());
    }

    #[test]
    fn constructs_from_reference_tree() {
        let root_id = Uuid::from_u128(1);
        let parent_resource_id = Uuid::from_u128(2);
        let child_resource_id = Uuid::from_u128(3);
        let ref_tree = RefTreeNode {
            entity_id: root_id,
            children: vec![RefTreeNode {
                entity_id: parent_resource_id,
                children: vec![RefTreeNode {
                    entity_id: child_resource_id,
                    children: vec![],
                }],
            }],
        };
        let resources = resources([parent_resource_id, child_resource_id]);

        let tree = ResourceTreeNode::try_from_ref_tree(ref_tree, &resources).unwrap();

        assert!(!tree.is_resource);
        assert!(tree.children[0].is_resource);
        assert!(tree.children[0].children[0].is_resource);
    }

    #[test]
    fn rejects_resource_absent_from_reference_tree() {
        let root_id = Uuid::from_u128(1);
        let resource_id = Uuid::from_u128(2);
        let ref_tree = RefTreeNode {
            entity_id: root_id,
            children: vec![],
        };
        let resources = resources([resource_id]);

        assert!(matches!(
            ResourceTreeNode::try_from_ref_tree(ref_tree, &resources),
            Err(AnalyzerError::Validation(_))
        ));
    }
}
