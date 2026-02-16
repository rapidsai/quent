//! Types shared with the UI.

use std::collections::HashMap;

use quent_query_engine_ui::{Engine, Operator, Plan, PlanTree, Port, Query, QueryGroup, Worker};
use quent_time::{TimeSec, TimeUnixNanoSec};
use quent_ui::{Resource, ResourceGroup, ResourceGroupTypeDecl, ResourceTypeDecl};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

pub mod timeline;

/// A reference to an entity
#[derive(TS, Debug, Serialize)]
pub enum EntityRef {
    Engine(Uuid),
    Worker(Uuid),
    QueryGroup(Uuid),
    Query(Uuid),
    Plan(Uuid),
    Operator(Uuid),
    Port(Uuid),
    Resource(Uuid),
    ResourceGroup(Uuid),
}

#[derive(TS, Debug, Serialize)]
pub struct QueryEntities {
    /// The engine that ran this query.
    ///
    /// Is a Resource Group.
    pub engine: Engine,
    /// The group of this query.
    ///
    /// Is a Resource Group.
    pub query_group: QueryGroup,
    /// The query.
    ///
    /// Is a Resource Group.
    pub query: Query,
    /// The workers of this query.
    ///
    /// Is a Resource Group.
    pub workers: HashMap<Uuid, Worker>,
    /// The plans of this query.
    ///
    /// Is a Resource Group.
    pub plans: HashMap<Uuid, Plan>,
    /// The operators of the plans.
    ///
    /// Is a Resource Group.
    pub operators: HashMap<Uuid, Operator>,
    /// The ports of the operators.
    ///
    /// Is a Resource Group.
    pub ports: HashMap<Uuid, Port>,

    /// Resource group types.
    ///
    /// This includes declarations for:
    /// - [`Engine`]
    /// - [`QueryGroup`]
    /// - [`Query`]
    /// - [`Worker`]
    /// - [`Plan`]
    /// - [`Operator`]
    /// - [`Port`]
    pub resource_group_types: HashMap<String, ResourceGroupTypeDecl>,

    /// Application-specific resources
    pub resources: HashMap<Uuid, Resource>,
    /// Application-specific resource types
    pub resource_types: HashMap<String, ResourceTypeDecl>,

    /// Application-specific resource groups
    pub resource_groups: HashMap<Uuid, ResourceGroup>,
}

#[derive(TS, Debug, Serialize)]
pub struct QueryBundle {
    /// The ID of the query.
    pub query_id: Uuid,
    /// Maps with entities that are involved in this query.
    pub entities: QueryEntities,

    /// A tree of plans involved in the execution of this query.
    pub plan_tree: PlanTree,
    /// A tree of resources involved in the execution of this query.
    pub resource_tree: ResourceTree,

    /// A list of unique operator type names.
    pub unique_operator_names: Vec<String>,

    /// The number of nanoseconds passed since the Unix epoch at which the
    /// engine started executing this query.
    pub start_time_unix_ns: TimeUnixNanoSec,
    /// The duration of this query, in seconds.
    pub duration_s: TimeSec,
}

/// A resource group node in a resource tree.
#[derive(TS, Debug, Serialize)]
pub struct ResourceGroupNode {
    pub id: EntityRef,
    pub children: Vec<ResourceTree>,
}

/// A tree of resources.
#[derive(TS, Debug, Serialize)]
pub enum ResourceTree {
    ResourceGroup(ResourceGroupNode),
    Resource(EntityRef),
}
