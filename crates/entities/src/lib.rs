//! Type definitions for model entities shared with the UI.
//!
//! These type definitions intentionally do not use generics, since many binding
//! generators will not support them.

use quent_events::engine::EngineImplementationAttributes;
use quent_events::resource::Scope;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::relation::Related;

pub mod error;
pub mod fsm;
pub mod relation;
pub mod resource;
pub mod timeline;

pub use error::{EntityError, Result};

// TODO(johanpel): figure out if we can stop being so verbose in prefixing type
// names with their namespace. This appears a limitation of ts_rs where you
// can't have two types of the same name in a different namespace. This is also
// a known limitation of e.g. wasm_bindgen.

/// Trait for entities that are not complete yet.
pub trait IncompleteEntity {
    fn new(id: Uuid) -> Self;
}

/// The total lifetime of an entity.
pub enum Lifetime {
    /// The entity is of a single event type, so it is only alive in one instant (as far as the model is concerned).
    Instant(TimeUnixNanoSec),
    /// The entity is alive across a span of time.
    Span(SpanUnixNanoSec),
}

/// Trait for entities that are complete.
pub trait Entity {
    fn id(&self) -> Uuid;
    fn lifetime(&self) -> Lifetime;
}

/// A run-time typed reference to an entity.
#[derive(TS, Clone, Copy, Debug, PartialEq, Eq, Serialize, Hash)]
pub enum EntityRef {
    // Domain-specific
    Engine(Uuid),
    QueryGroup(Uuid),
    Query(Uuid),
    Plan(Uuid),
    Worker(Uuid),
    Operator(Uuid),
    Port(Uuid),
    // Generic
    ResourceGroup(Uuid),
    Resource(Uuid),
    Fsm(Uuid),
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
            EntityRef::Fsm(uuid) => uuid,
        }
    }
}

pub mod engine {
    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of an
    /// Engine.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct EngineTimestamps {
        /// The time at which the Engine started initialization.
        pub init: Option<TimeUnixNanoSec>,
        /// The time at which the Engine started accepting queries.
        pub operating: Option<TimeUnixNanoSec>,
        /// The time at which the Engine started shutting down and cleaning up
        /// its resources.
        pub finalizing: Option<TimeUnixNanoSec>,
        /// The time at which the Engine was completely destructed and all
        /// resources were freed.
        pub exit: Option<TimeUnixNanoSec>,
    }

    /// An Engine represents the top-level entity of the model.
    ///
    /// Engines accept Queries that they pass to Query Groups which in turn
    /// orchestrate execution through Plans submitted to Workers.
    ///
    /// Nothing can outlive the lifetime of an Engine. TODO(johanpel): this
    /// assumes 0 clock skew, we need to address this in general.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct Engine {
        /// The ID of this Engine
        pub id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the
        /// Engine.
        #[serde(skip)]
        #[ts(skip)]
        pub timestamps: EngineTimestamps,

        /// The name of this Engine - typically a name for this instance of a
        /// specific engine implementation.
        pub name: Option<String>,
        /// Details about the Engine implementation.
        pub implementation: Option<EngineImplementationAttributes>,
    }

    impl IncompleteEntity for Engine {
        fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }

    impl Related for Engine {
        fn relations(&self) -> impl Iterator<Item = EntityRef> {
            std::iter::empty()
        }
    }
}

pub mod query_group {
    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// QueryGroup.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct QueryGroupTimestamps {
        /// The time at which the QueryGroup started initialization.
        pub init: Option<TimeUnixNanoSec>,
        /// The time at which the QueryGroup started accepting queries.
        pub operating: Option<TimeUnixNanoSec>,
        /// The time at which the QueryGroup started shutting down and cleaning
        /// up its resources.
        pub finalizing: Option<TimeUnixNanoSec>,
        /// The time at which the QueryGroup was completely destructed and all
        /// resources were freed.
        pub exit: Option<TimeUnixNanoSec>,
    }

    /// A QueryGroup is an entity that orchestrates the execution of a distinct
    /// set of queries.
    ///
    /// For example, a session in a long-lived multi-user engine could be
    /// modeled as a QueryGroup. TODO(johanpel): perhaps this isn't a great name
    /// for this concept, consider naming this something else.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct QueryGroup {
        /// The ID of this QueryGroup
        pub id: Uuid,
        /// The ID of the Engine this QueryGroup was spawned in
        pub engine_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the
        /// QueryGroup.
        #[serde(skip)]
        #[ts(skip)]
        pub timestamps: QueryGroupTimestamps,
        /// A name for this QueryGroup instance
        pub name: Option<String>,
    }

    impl IncompleteEntity for QueryGroup {
        fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }

    impl Related for QueryGroup {
        fn relations(&self) -> impl Iterator<Item = EntityRef> {
            [EntityRef::Engine(self.engine_id)].into_iter()
        }
    }
}

pub mod worker {
    use crate::relation::Related;

    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// Worker.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct WorkerTimestamps {
        /// The time at which the Worker started initialization.
        pub init: Option<TimeUnixNanoSec>,
        /// The time at which the Worker started accepting Plans.
        pub operating: Option<TimeUnixNanoSec>,
        /// The time at which the Worker started shutting down and cleaning up
        /// its resources.
        pub finalizing: Option<TimeUnixNanoSec>,
        /// The time at which the Worker was completely destructed and all
        /// resources were freed.
        pub exit: Option<TimeUnixNanoSec>,
    }

    /// A Worker is an entity that executes Query Plans.
    ///
    /// It is a high-level resource of an Engine. Its lifetime is bounded by the
    /// lifetime of an Engine, but it can outlive any other entity.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct Worker {
        /// The ID of this Worker.
        pub id: Uuid,
        /// The ID of the Engine that spawned this Worker.
        pub engine_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the
        /// Worker.
        #[serde(skip)]
        #[ts(skip)]
        pub timestamps: WorkerTimestamps,
        /// A name for this Worker instance
        pub name: Option<String>,
    }

    impl IncompleteEntity for Worker {
        fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }

    impl Related for Worker {
        fn relations(&self) -> impl Iterator<Item = EntityRef> {
            [EntityRef::Engine(self.engine_id)].into_iter()
        }
    }
}

pub mod query {
    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// Query.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct QueryTimestamps {
        /// The time at which the Query started initialization.
        pub init: Option<TimeUnixNanoSec>,
        /// The time at which the Query started planning.
        pub planning: Option<TimeUnixNanoSec>,
        /// The time at which the Query started executing.
        pub executing: Option<TimeUnixNanoSec>,
        /// The time at which the Query was idle.
        ///
        /// In this state, the Query has been processed, but it still
        /// potentially occupies resources of the engine to hold a result which
        /// is yet to be delivered to the query client.
        pub idle: Option<TimeUnixNanoSec>,
        /// The time at which the Query started shutting down and cleaning up
        /// its resources.
        pub finalizing: Option<TimeUnixNanoSec>,
        /// The time at which the Query was completely destructed and all
        /// resources were freed.
        pub exit: Option<TimeUnixNanoSec>,
    }

    /// A Query.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct Query {
        /// The ID of this Query
        pub id: Uuid,
        /// The ID of the QueryGroup orchestrating this query
        pub query_group_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the
        /// Query.
        #[serde(skip)]
        #[ts(skip)]
        pub timestamps: QueryTimestamps,
        /// A name for this Query.
        pub name: Option<String>,
    }

    impl IncompleteEntity for Query {
        fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }

    impl Related for Query {
        fn relations(&self) -> impl Iterator<Item = EntityRef> {
            [EntityRef::QueryGroup(self.query_group_id)].into_iter()
        }
    }
}

pub mod operator {
    use super::*;

    /// A state transition where an Operator is blocked from progressing
    /// beceause it is waiting for inputs to arrive.
    #[derive(TS, Clone, Debug, Serialize)]
    pub struct WaitingForInputs {
        /// The timestamp of this transition.
        pub timestamp: TimeUnixNanoSec,
        /// The IDs of the Ports this Operator was blocked on.
        pub ports: Vec<Uuid>,
    }

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a
    /// Query.
    #[derive(TS, Clone, Debug, Serialize)]
    pub enum OperatorState {
        /// The Operator is initializing, allocating its resources.
        Init(TimeUnixNanoSec),
        /// The Operator is waiting for inputs.
        WaitingForInputs(WaitingForInputs),
        /// The Operator is actively processing.
        Executing(TimeUnixNanoSec),
        /// The Operator is blocked.
        Blocked(TimeUnixNanoSec),
        /// The Operator is finalizing, cleaning up its resources.
        Finalizing(TimeUnixNanoSec),
        /// The Operator is completely finalized, and no longer holds any
        /// resources.
        Exit(TimeUnixNanoSec),
    }

    impl OperatorState {
        pub fn timestamp(&self) -> TimeUnixNanoSec {
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
    #[derive(TS, Clone, Debug, Default, Serialize)]
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
        #[serde(skip)]
        #[ts(skip)]
        pub state_sequence: Vec<OperatorState>,
    }

    impl IncompleteEntity for Operator {
        fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }

    impl Related for Operator {
        fn relations(&self) -> impl Iterator<Item = EntityRef> {
            [EntityRef::Plan(self.parent_plan_id)].into_iter()
        }
    }

    /// A Port of an Operator in a Plan DAG.
    ///
    /// Note a Port is not an FSM so none of its non-id fields need to be
    /// optional as they are declared within a single event.
    #[derive(TS, Clone, Default, Debug, Serialize)]
    pub struct Port {
        pub id: Uuid,
        pub parent_operator_id: Uuid,
        pub name: String,
    }

    impl IncompleteEntity for Port {
        fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }

    impl Related for Port {
        fn relations(&self) -> impl Iterator<Item = EntityRef> {
            [EntityRef::Operator(self.parent_operator_id)].into_iter()
        }
    }
}

pub mod plan {
    use quent_events::plan::Edge;

    use super::*;

    /// Timestamps of plan state transitions.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct PlanTimestamps {
        /// The time at which the Plan started initialization.
        pub init: Option<TimeUnixNanoSec>,
        /// The time at which the Plan started execution.
        pub executing: Option<TimeUnixNanoSec>,
        /// The time at which Plan execution was completed.
        pub idle: Option<TimeUnixNanoSec>,
        /// The time at which the Plan started cleaning up its resources.
        pub finalizing: Option<TimeUnixNanoSec>,
        /// The time at which the Worker was completely destructed and all
        /// resources were freed.
        pub exit: Option<TimeUnixNanoSec>,
    }

    /// A Query Plan.
    #[derive(TS, Clone, Debug, Default, Serialize)]
    pub struct Plan {
        /// The ID of this Plan.
        pub id: Uuid,
        /// The name of this Plan.
        pub name: Option<String>,
        /// The ID of the Query this Plan is part of.
        pub query_id: Option<Uuid>,
        /// The timestamps of various state transitions during the lifetime of
        /// this plan.
        #[serde(skip)]
        #[ts(skip)]
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

    impl IncompleteEntity for Plan {
        fn new(id: Uuid) -> Self {
            Self {
                id,
                ..Default::default()
            }
        }
    }

    impl Related for Plan {
        fn relations(&self) -> impl Iterator<Item = EntityRef> {
            if let Some(parent) = self.parent_plan_id {
                vec![EntityRef::Plan(parent)].into_iter()
            } else {
                vec![].into_iter()
            }
        }
    }
}
