use std::collections::HashMap;

use quent_entities::EntityRef;
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{Entities, Result, error::Error, plan_tree::PlanTree};

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
    /// Form a ResourceTree from the resources directly under some scope.
    fn from_scope(entities: &Entities, entity: EntityRef) -> Self {
        let children = entities
            .get_resources_within_scope(entity)
            .into_iter()
            .map(|entity| Self::from_scope(entities, entity))
            .collect::<Vec<_>>();
        Self {
            item: Some(entity),
            children,
        }
    }

    pub(crate) fn try_new(
        entities: &Entities,
        plan_tree: &PlanTree,
        query_id: Uuid,
    ) -> Result<Self> {
        let query = entities
            .queries
            .get(&query_id)
            .ok_or(Error::InvalidId(query_id))?;
        let query_group = entities
            .query_groups
            .get(&query.query_group_id)
            .ok_or(Error::Logic(format!(
                "unable to construct resource tree - query group {} of query {} does not exist",
                query.query_group_id, query.id
            )))?;

        let engine_node = ResourceTree::from_scope(entities, EntityRef::Engine(entities.engine.id));
        let query_group_node =
            ResourceTree::from_scope(entities, EntityRef::QueryGroup(query_group.id));
        let query_node = ResourceTree::from_scope(entities, EntityRef::Query(query_id));

        // Traverse the plan tree DFS in order to figure out which plans can go
        // under query, and which ones can go under worker. Since we're doing
        // this DFS, the order of lineage of plans is retained.
        let mut query_plan_nodes = vec![];
        let mut worker_plan_ids: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        plan_tree.visit_depth_first(&mut |p| {
            // If this plan is not worker-local, push it under the global part
            // of the resource tree.
            if let (Some(q), None) = (p.query, p.worker) {
                // Sanity check.
                if q != query_id {
                    return Err(Error::Logic(format!(
                        "Unexpected reference to query {q} not in entities."
                    )));
                }
                query_plan_nodes.push(ResourceTree::from_scope(entities, EntityRef::Plan(p.id)));
            } else if let Some(w) = p.worker {
                worker_plan_ids.entry(w).or_default().push(p.id);
            }
            Ok(())
        })?;

        // Now iterate over all workers, and nest their plans.
        let mut worker_nodes = entities
            .workers
            .keys()
            .map(|k| {
                (
                    *k,
                    ResourceTree::from_scope(entities, EntityRef::Worker(*k)),
                )
            })
            .collect::<HashMap<_, _>>();

        for (worker_id, worker_plan_ids) in worker_plan_ids {
            let entry = worker_nodes.entry(worker_id).or_insert_with(|| {
                ResourceTree::from_scope(entities, EntityRef::Worker(worker_id))
            });
            entry.children.extend(
                worker_plan_ids
                    .into_iter()
                    .map(|plan_id| ResourceTree::from_scope(entities, EntityRef::Plan(plan_id))),
            );
        }

        let mut children = vec![engine_node, query_group_node, query_node];
        children.extend(query_plan_nodes);
        children.extend(worker_nodes.into_values());

        Ok(ResourceTree {
            item: None,
            children,
        })
    }
}
