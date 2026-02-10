//! Types shared with the UI.

use std::collections::HashMap;

use quent_query_engine_ui::{Engine, Operator, Plan, PlanTree, Port, Query, QueryGroup, Worker};
use quent_time::{TimeSec, TimeUnixNanoSec, bin::BinnedSpanSec};
use quent_ui::{Resource, ResourceGroup, ResourceTypeDecl};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

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
    pub engine: Engine,
    /// The group of this query.
    pub query_group: QueryGroup,
    /// The query.
    pub query: Query,
    /// The workers of this query.
    pub workers: HashMap<Uuid, Worker>,
    /// The plans of this query.
    pub plans: HashMap<Uuid, Plan>,
    /// The operators of the plans.
    pub operators: HashMap<Uuid, Operator>,
    /// The ports of the operators
    pub ports: HashMap<Uuid, Port>,
    /// Miscellaneous resources
    pub resources: HashMap<Uuid, Resource>,
    /// Miscellaneous resource groups
    pub resource_groups: HashMap<Uuid, ResourceGroup>,
    /// Miscellaneous resource types
    pub resource_types: HashMap<String, ResourceTypeDecl>,
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

#[derive(TS, Debug, Serialize)]
pub struct ResourceTimelineBinned {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpanSec,
    /// Maps a resource capacity name to a vector where each element holds an
    /// aggregated value of a time bin.
    pub capacities_values: HashMap<String, Vec<f64>>,
}

#[derive(TS, Debug, Serialize)]
pub struct ResourceTimelineBinnedByState {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpanSec,
    /// Maps a resource capacity name to a map of a state name to a vector where
    /// each element holds an aggregated value of a time bin.
    pub capacities_states_values: HashMap<String, HashMap<String, Vec<f64>>>,
}

#[derive(TS, Debug, Serialize)]
pub enum TimelineResponse {
    Binned(ResourceTimelineBinned),
    BinnedByState(ResourceTimelineBinnedByState),
}

#[derive(TS, Debug, Deserialize)]
pub struct ResourceTimelineUrlQueryParams {
    /// The number of bins.
    ///
    /// u16::MAX is large enough when bins are plotted as single pixel wide
    /// bars, even for insane screen resolutions.
    pub num_bins: u16,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,

    /// If set, only include utilizations from FSMs with this type name, and
    /// aggregate for each state separately.
    ///
    /// Can be set for both resource and resource group timelines.
    pub fsm_type_name: Option<String>,

    /// Sets the resource type for which to provide an aggregated timeline.
    ///
    /// This is required for resource group routes, and is ignored for
    /// individual resource timeline routes.
    pub resource_type_name: Option<String>,
}
