//! Type definitions for entities of the model.
//!
//! These type definitions intentionally do not use generics, since many binding
//! generators will not support them.
use py_rs::PY;
use quent_events::{Timestamp, resource::Scope};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

// TODO(johanpel): figure out if we can stop being so verbose in prefixing type
// names with their namespace. This appears a limitation of ts_rs where you
// can't have two types of the same name in a different namespace. This is also
// a known limitation of e.g. wasm_bindgen.

/// A run-time typed reference to an entity.
#[derive(TS, PY, Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum EntityRef {
    Engine(Uuid),
    QueryGroup(Uuid),
    Query(Uuid),
    Plan(Uuid),
    Worker(Uuid),
    Operator(Uuid),
    Port(Uuid),
    ResourceGroup(Uuid),
    Resource(Uuid),
}

impl From<Scope> for EntityRef {
    fn from(value: Scope) -> Self {
        match value {
            Scope::Engine(uuid) => EntityRef::Engine(uuid),
            Scope::QueryGroup(uuid) => EntityRef::QueryGroup(uuid),
            Scope::Query(uuid) => EntityRef::Query(uuid),
            Scope::Plan(uuid) => EntityRef::Plan(uuid),
            Scope::Worker(uuid) => EntityRef::Worker(uuid),
            Scope::Operator(uuid) => EntityRef::Operator(uuid),
            Scope::Port(uuid) => EntityRef::Port(uuid),
            Scope::ResourceGroup(uuid) => EntityRef::ResourceGroup(uuid),
        }
    }
}

impl From<EntityRef> for Uuid {
    fn from(value: EntityRef) -> Self {
        match value {
            EntityRef::Engine(uuid) => uuid,
            EntityRef::QueryGroup(uuid) => uuid,
            EntityRef::Query(uuid) => uuid,
            EntityRef::Plan(uuid) => uuid,
            EntityRef::Worker(uuid) => uuid,
            EntityRef::Operator(uuid) => uuid,
            EntityRef::Port(uuid) => uuid,
            EntityRef::ResourceGroup(uuid) => uuid,
            EntityRef::Resource(uuid) => uuid,
        }
    }
}

pub mod engine {
    use quent_events::engine::EngineImplementationAttributes;

    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of an
    /// Engine.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct EngineTimestamps {
        /// The time at which the Engine started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Engine started accepting queries.
        pub operating: Option<Timestamp>,
        /// The time at which the Engine started shutting down and cleaning up
        /// its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Engine was completely destructed and all
        /// resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// An Engine represents the top-level entity of the model.
    ///
    /// Engines accept Queries that they pass to Query Groups which in turn
    /// orchestrate execution through Plans submitted to Workers.
    ///
    /// Nothing can outlive the lifetime of an Engine. TODO(johanpel): this
    /// assumes 0 clock skew, we need to address this in general.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Engine {
        /// The ID of this Engine
        pub id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the
        /// Engine.
        pub timestamps: EngineTimestamps,

        /// The name of this Engine - typically a name for this instance of a
        /// specific engine implementation.
        pub name: Option<String>,
        /// Details about the Engine implementation.
        pub implementation: Option<EngineImplementationAttributes>,
    }

    impl Engine {
        pub fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }
}

pub mod query_group {
    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// QueryGroup.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct QueryGroupTimestamps {
        /// The time at which the QueryGroup started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the QueryGroup started accepting queries.
        pub operating: Option<Timestamp>,
        /// The time at which the QueryGroup started shutting down and cleaning
        /// up its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the QueryGroup was completely destructed and all
        /// resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// A QueryGroup is an entity that orchestrates the execution of a distinct
    /// set of queries.
    ///
    /// For example, a session in a long-lived multi-user engine could be
    /// modeled as a QueryGroup. TODO(johanpel): perhaps this isn't a great name
    /// for this concept, consider naming this something else.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct QueryGroup {
        /// The ID of this QueryGroup
        pub id: Uuid,
        /// The ID of the Engine this QueryGroup was spawned in
        pub engine_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the
        /// QueryGroup.
        pub timestamps: QueryGroupTimestamps,
        /// A name for this QueryGroup instance
        pub name: Option<String>,
    }

    impl QueryGroup {
        pub fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }
}

pub mod worker {
    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// Worker.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct WorkerTimestamps {
        /// The time at which the Worker started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Worker started accepting Plans.
        pub operating: Option<Timestamp>,
        /// The time at which the Worker started shutting down and cleaning up
        /// its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Worker was completely destructed and all
        /// resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// A Worker is an entity that executes Query Plans.
    ///
    /// It is a high-level resource of an Engine. Its lifetime is bounded by the
    /// lifetime of an Engine, but it can outlive any other entity.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Worker {
        /// The ID of this Worker.
        pub id: Uuid,
        /// The ID of the Engine that spawned this Worker.
        pub engine_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the
        /// Worker.
        pub timestamps: WorkerTimestamps,
        /// A name for this Worker instance
        pub name: Option<String>,
    }

    impl Worker {
        pub fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }
}

pub mod query {
    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// Query.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct QueryTimestamps {
        /// The time at which the Query started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Query started planning.
        pub planning: Option<Timestamp>,
        /// The time at which the Query started executing.
        pub executing: Option<Timestamp>,
        /// The time at which the Query was idle.
        ///
        /// In this state, the Query has been processed, but it still
        /// potentially occupies resources of the engine to hold a result which
        /// is yet to be delivered to the query client.
        pub idle: Option<Timestamp>,
        /// The time at which the Query started shutting down and cleaning up
        /// its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Query was completely destructed and all
        /// resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// A Query.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Query {
        /// The ID of this Query
        pub id: Uuid,
        /// The ID of the QueryGroup orchestrating this query
        pub query_group_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the
        /// Query.
        pub timestamps: QueryTimestamps,
        /// A name for this Query.
        pub name: Option<String>,
    }

    impl Query {
        pub fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }
}

pub mod operator {
    use super::*;

    /// A state transition where an Operator is blocked from progressing
    /// beceause it is waiting for inputs to arrive.
    #[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
    pub struct WaitingForInputs {
        /// The timestamp of this transition.
        pub timestamp: Timestamp,
        /// The IDs of the Ports this Operator was blocked on.
        pub ports: Vec<Uuid>,
    }

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// Query.
    #[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
    pub enum OperatorState {
        /// The Operator is initializing, allocating its resources.
        Init(Timestamp),
        /// The Operator is waiting for inputs.
        WaitingForInputs(WaitingForInputs),
        /// The Operator is actively processing.
        Executing(Timestamp),
        /// The Operator is blocked.
        Blocked(Timestamp),
        /// The Operator is finalizing, cleaning up its resources.
        Finalizing(Timestamp),
        /// The Operator is completely finalized, and no longer holds any
        /// resources.
        Exit(Timestamp),
    }

    impl OperatorState {
        pub fn timestamp(&self) -> Timestamp {
            match self {
                OperatorState::Init(ts) => *ts,
                OperatorState::WaitingForInputs(waiting_for_inputs) => waiting_for_inputs.timestamp,
                OperatorState::Executing(ts) => *ts,
                OperatorState::Blocked(ts) => *ts,
                OperatorState::Finalizing(ts) => *ts,
                OperatorState::Exit(ts) => *ts,
            }
        }
    }

    /// An Operator in a Plan DAG.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Operator {
        /// The ID of this Operator.
        pub id: Uuid,
        /// The ID of the Plan this Operator belongs to.
        pub parent_plan_id: Uuid,
        /// A list of Operator IDs in a parent plan (if any) from which this
        /// Operator was derived.
        pub parent_operator_ids: Vec<Uuid>,
        /// The name of this Operator.
        pub name: Option<String>,
        /// The IDs of the Ports of this operator.
        pub ports: Vec<Uuid>,
        /// The sequence of states through which this Operator has been
        /// executed.
        pub state_sequence: Vec<OperatorState>,
    }

    /// A Port of an Operator in a Plan DAG.
    #[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
    pub struct Port {
        pub id: Uuid,
        pub parent_operator_id: Uuid,
        pub name: String,
    }
}

pub mod plan {
    use quent_events::plan::Edge;

    use super::*;

    /// Timestamps of plan state transitions.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct PlanTimestamps {
        /// The time at which the Plan started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Plan started execution.
        pub executing: Option<Timestamp>,
        /// The time at which Plan execution was completed.
        pub idle: Option<Timestamp>,
        /// The time at which the Plan started cleaning up its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Worker was completely destructed and all
        /// resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// A Query Plan.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Plan {
        /// The ID of this Plan.
        pub id: Uuid,
        /// The name of this Plan.
        pub name: String,
        /// The ID of the Query this Plan is part of.
        pub query_id: Uuid,
        /// The timestamps of various state transitions during the lifetime of
        /// this plan.
        pub timestamps: PlanTimestamps,
        /// The optional parent Plan ID. This is useful if an Engine constructs
        /// various types of Plans before execution, sometimes referred to as
        /// "lowering". Examples include a logical and physical plan.
        pub parent_plan_id: Option<Uuid>,
        /// The optional Worker ID of the Worker that executed this Plan.
        ///
        /// If this Plan was not directly executed on a worker, but merely some
        /// level plan in a sequence of lowering stages, then this may be set to
        /// None.
        pub worker_id: Option<Uuid>,
        /// The IDs of the Operators that are part of this Plan.
        pub operator_ids: Vec<Uuid>,
        /// The Edges between Operators of this Plan.
        pub edges: Vec<Edge>,
    }
}

pub mod resource {
    use quent_events::attributes::Attribute;

    use super::*;

    /// Attributes of the "Operating" state of a Resource.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct ResourceOperatingState {
        pub timestamp: Timestamp,
        pub capacities: Vec<Attribute>,
    }

    /// Resource states.
    #[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
    pub enum ResourceState {
        Init(Timestamp),
        Operating(ResourceOperatingState),
        Resizing(Timestamp),
        Finalizing(Timestamp),
        Exit(Timestamp),
    }

    /// A Resource of which its Capacities cannot be resized.
    #[derive(TS, PY, Clone, Default, Debug, Deserialize, Serialize)]
    pub struct Resource {
        /// The ID of this Resource
        pub id: Uuid,
        /// The name of this Resource
        pub name: Option<String>,
        /// The scope of this Resource.
        pub scope: Option<EntityRef>,
        /// The Capacities of this Resource.
        ///
        /// If this is empty, this is Unit resource.
        pub capacities: Vec<Attribute>,
        /// The sequence of states that this Resource
        pub state_sequence: Vec<ResourceState>,
    }

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// ResourceGroup.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct ResourceGroupTimestamps {
        /// The time at which the ResourceGroup started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the ResourceGroup started operating.
        pub operating: Option<Timestamp>,
        /// The time at which the ResourceGroup started shutting down and
        /// cleaning up its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the ResourceGroup was completely destructed and
        /// all resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// A Group of Resources.
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct ResourceGroup {
        /// The ID of this Resource Group
        pub id: Uuid,
        /// The name of this Resource Group
        pub name: String,
        /// The scope of this Resource Group
        pub scope: Option<EntityRef>,
        /// The timestamps of state transitions of this ResourceGroup.
        pub timestamps: ResourceGroupTimestamps,
    }
}
