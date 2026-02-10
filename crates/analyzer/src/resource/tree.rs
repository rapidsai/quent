//! Functions to construct the [`ResourceTree`] of an application model

use uuid::Uuid;

use crate::{AnalyzerResult, resource::collection::ResourceCollection};

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

    /// Return an iterator over to all [`quent_entities::resource::Resource`]
    /// leaves under this tree.
    pub(crate) fn iter_leaves(&self) -> ResourceTreeLeafIter<'_> {
        ResourceTreeLeafIter { stack: vec![self] }
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
