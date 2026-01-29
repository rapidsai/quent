use quent_entities::EntityRef;
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{AnalyzerResult, Entities, error::AnalyzerError};

/// A tree of references to entities that can hold resources in their scope,
/// with resources as leaf nodes.
///
/// The tree is built from the perspective of a single query. Thus, under the
/// root level, engine, query group and query and query-level plans are
/// siblings.
#[derive(TS, Clone, Debug, Serialize)]
pub struct ResourceTree {
    /// The ID of the Entity representing this node.
    ///
    /// This is either an [`EntityRef::Resource`] or a
    /// [`EntityRef::ResourceGroup`].
    ///
    /// If this is None, this is the root level.
    pub item: Option<EntityRef>,
    /// The children of this "resource".
    ///
    /// Valid for all items except those of the variant
    /// ResourceTreeNodeItem::Resource
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ResourceTree>,
}

impl ResourceTree {
    pub(crate) fn try_new(entities: &Entities, root_group_id: Uuid) -> AnalyzerResult<Self> {
        Ok(ResourceTree {
            item: Some(EntityRef::ResourceGroup(root_group_id)),
            children: entities
                .resource_group_children(root_group_id)?
                .map(|child| match child {
                    EntityRef::ResourceGroup(uuid) => Self::try_new(entities, uuid),
                    EntityRef::Resource(_) => Ok(Self {
                        item: Some(child),
                        children: vec![],
                    }),
                    _ => Err(AnalyzerError::BrokenImpl(
                        "unexpected non-resource(group) types".to_string(),
                    )),
                })
                .collect::<AnalyzerResult<_>>()?,
        })
    }

    /// Return an iterator over to all [`quent_entities::resource::Resource`]
    /// leaves under this tree.
    pub(crate) fn iter_leaves(&self) -> ResourceTreeLeafIter<'_> {
        ResourceTreeLeafIter { stack: vec![self] }
    }
}

/// Iterator over the leaf resources of a [`ResourceTree`].
pub struct ResourceTreeLeafIter<'a> {
    stack: Vec<&'a ResourceTree>,
}

impl<'a> Iterator for ResourceTreeLeafIter<'a> {
    type Item = Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            self.stack.extend(node.children.iter().rev());
            if let Some(EntityRef::Resource(uuid)) = node.item {
                return Some(uuid);
            }
        }
        None
    }
}
