// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Functions to construct the [`ResourceTree`] of an application model

use std::collections::VecDeque;

use uuid::Uuid;

use crate::{
    AnalyzerResult,
    resource::{Resource, collection::ResourceCollection},
};

pub enum ResourceTreeNode {
    ResourceGroup(Uuid, Vec<ResourceTreeNode>),
    Resource(Uuid),
}

impl ResourceTreeNode {
    pub fn try_new(
        resources: &impl ResourceCollection,
        root_group_id: Uuid,
    ) -> AnalyzerResult<Self> {
        let group_children = resources
            .resource_group_child_groups(root_group_id)?
            .map(|child_group| Self::try_new(resources, child_group));
        let resource_children = resources
            .resource_group_child_resources(root_group_id)?
            .map(|child_resource| Ok(ResourceTreeNode::Resource(child_resource)));
        Ok(ResourceTreeNode::ResourceGroup(
            root_group_id,
            group_children
                .chain(resource_children)
                .collect::<AnalyzerResult<_>>()?,
        ))
    }

    /// Return an iterator over to all ids of leaf [`Resource`]s.
    pub fn iter_leaf_ids(&self) -> ResourceTreeLeafIter<'_> {
        ResourceTreeLeafIter { stack: vec![self] }
    }

    /// Return an iterator over references to all leaf [`Resource`]s.
    pub fn iter_leaf_refs<'a>(
        &self,
        resources: &'a impl ResourceCollection,
    ) -> impl Iterator<Item = AnalyzerResult<&'a dyn Resource>> {
        self.iter_leaf_ids().map(|id| resources.resource(id))
    }

    /// Return an iterator over all group IDs in this tree (including the root).
    pub fn iter_group_ids(&self) -> ResourceTreeGroupIter<'_> {
        ResourceTreeGroupIter { stack: vec![self] }
    }

    /// Breadth-first-search for a specific resource or resource group id.
    pub fn find(&self, target_id: Uuid) -> Option<&Self> {
        let mut queue = VecDeque::new();
        queue.push_back(self);

        while let Some(node) = queue.pop_front() {
            match node {
                ResourceTreeNode::ResourceGroup(id, children) => {
                    if *id == target_id {
                        return Some(node);
                    }
                    queue.extend(children.iter());
                }
                ResourceTreeNode::Resource(id) => {
                    if *id == target_id {
                        return Some(node);
                    }
                }
            }
        }

        None
    }
}

/// Iterator over the leaf resources of a [`ResourceTree`].
pub struct ResourceTreeLeafIter<'a> {
    stack: Vec<&'a ResourceTreeNode>,
}

impl<'a> Iterator for ResourceTreeLeafIter<'a> {
    type Item = Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                ResourceTreeNode::ResourceGroup(_, children) => {
                    self.stack.extend(children.iter().rev());
                }
                ResourceTreeNode::Resource(uuid) => {
                    return Some(*uuid);
                }
            }
        }
        None
    }
}

/// Iterator over all resource groups in a [`ResourceTree`].
pub struct ResourceTreeGroupIter<'a> {
    stack: Vec<&'a ResourceTreeNode>,
}

impl<'a> Iterator for ResourceTreeGroupIter<'a> {
    type Item = Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                ResourceTreeNode::ResourceGroup(uuid, children) => {
                    self.stack.extend(children.iter().rev());
                    return Some(*uuid);
                }
                ResourceTreeNode::Resource(_) => {
                    // Skip leaf resources
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn iter_leaf_ids_flat() {
        let r1 = Uuid::from_u64_pair(1, 1);
        let r2 = Uuid::from_u64_pair(1, 2);
        let g1 = Uuid::from_u64_pair(2, 1);

        let tree = ResourceTreeNode::ResourceGroup(
            g1,
            vec![
                ResourceTreeNode::Resource(r1),
                ResourceTreeNode::Resource(r2),
            ],
        );

        let leaves: Vec<_> = tree.iter_leaf_ids().collect();
        assert_eq!(leaves, vec![r1, r2]);
    }

    #[test]
    fn iter_leaf_ids_nested() {
        let r1 = Uuid::from_u64_pair(1, 1);
        let r2 = Uuid::from_u64_pair(1, 2);
        let g1 = Uuid::from_u64_pair(2, 1);
        let g2 = Uuid::from_u64_pair(2, 2);

        let tree = ResourceTreeNode::ResourceGroup(
            g1,
            vec![
                ResourceTreeNode::Resource(r1),
                ResourceTreeNode::ResourceGroup(g2, vec![ResourceTreeNode::Resource(r2)]),
            ],
        );

        let leaves: Vec<_> = tree.iter_leaf_ids().collect();
        assert_eq!(leaves, vec![r1, r2]);
    }

    #[test]
    fn iter_group_ids() {
        let r1 = Uuid::from_u64_pair(1, 1);
        let g1 = Uuid::from_u64_pair(2, 1);
        let g2 = Uuid::from_u64_pair(2, 2);

        let tree = ResourceTreeNode::ResourceGroup(
            g1,
            vec![ResourceTreeNode::ResourceGroup(
                g2,
                vec![ResourceTreeNode::Resource(r1)],
            )],
        );

        let groups: Vec<_> = tree.iter_group_ids().collect();
        assert_eq!(groups, vec![g1, g2]);
    }
}
