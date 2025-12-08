//! Type definitions for entities of the model.
use quent_events::Timestamp;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

// TODO(johanpel): figure out if we can stop being so verbose in prefixing type names with their
//                 namespace. This appears a limitation of ts_rs where you can't have two types
//                 of the same name in a different namespace.

pub mod engine {
    use quent_events::engine::EngineImplementationAttributes;

    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of an Engine.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct EngineTimestamps {
        /// The time at which the Engine started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Engine started accepting queries.
        pub operating: Option<Timestamp>,
        /// The time at which the Engine started shutting down and cleaning up its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Engine was completely destructed and all resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// An Engine represents the top-level entity of the model.
    ///
    /// Engines accept Queries that they pass to Coordinators which in turn orchestrates
    /// execution through Plans submitted to Workers.
    ///
    /// Nothing can outlive the lifetime of an Engine.
    /// TODO(johanpel): this assumes 0 clock skew, we need to address this in general.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Engine {
        /// The ID of this Engine
        pub id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the Engine.
        pub timestamps: EngineTimestamps,

        /// The name of this Engine - typically a name for this instance of a specific engine implementation.
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

pub mod coordinator {
    use super::*;

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a Coordinator.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct CoordinatorTimestamps {
        /// The time at which the Coordinator started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Coordinator started accepting queries.
        pub operating: Option<Timestamp>,
        /// The time at which the Coordinator started shutting down and cleaning up its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Coordinator was completely destructed and all resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// A Coordinator is an entity that orchestrates the execution of a distinct set of queries.
    ///
    /// For example, a session in a long-lived multi-user engine could be modeled as a Coordinator.
    /// TODO(johanpel): perhaps this isn't a great name for this concept, consider naming this something else.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Coordinator {
        /// The ID of this Coordinator
        pub id: Uuid,
        /// The ID of the Engine this Coordinator was spawned in
        pub engine_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the Coordinator.
        pub timestamps: CoordinatorTimestamps,
        /// A name for this Coordinator instance
        pub name: Option<String>,
    }

    impl Coordinator {
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

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a Worker.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct WorkerTimestamps {
        /// The time at which the Worker started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Worker started accepting Plans.
        pub operating: Option<Timestamp>,
        /// The time at which the Worker started shutting down and cleaning up its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Worker was completely destructed and all resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// A Worker is an entity that executes Query Plans.
    ///
    /// It is a high-level resource of an Engine.
    /// Its lifetime is bounded by the lifetime of an Engine, but it can outlive any other entity.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Worker {
        /// The ID of this Worker.
        pub id: Uuid,
        /// The ID of the Engine that spawned this Worker.
        pub engine_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the Worker.
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

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a Query.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct QueryTimestamps {
        /// The time at which the Query started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Query started planning.
        pub planning: Option<Timestamp>,
        /// The time at which the Query started executing.
        pub executing: Option<Timestamp>,
        /// The time at which the Query was idle.
        ///
        /// In this state, the Query has been processed, but it still potentially occupies
        /// resources of the engine to hold a result which is yet to be delivered to the query
        /// client.
        pub idle: Option<Timestamp>,
        /// The time at which the Query started shutting down and cleaning up its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Query was completely destructed and all resources were freed.
        pub exit: Option<Timestamp>,
    }

    /// A Coordinator is an entity that orchestrates the execution of a distinct set of queries.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Query {
        /// The ID of this Query
        pub id: Uuid,
        /// The ID of the Coordinator orchestrating this query
        pub coordinator_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the Query.
        pub timestamps: QueryTimestamps,
        /// A name for this Query.
        pub name: Option<String>,
        /// The plans of this Query.
        pub plans: Vec<plan::Plan>,
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

    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
    pub struct WaitingForInputs {
        pub timestamp: Timestamp,
        pub ports: Vec<Uuid>,
    }

    /// Timestamps (nanoseconds since Unix epoch) of state transitions of a Query.
    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
    pub enum OperatorState {
        Init(Timestamp),
        /// The Operator is waiting for inputs.
        WaitingForInputs(WaitingForInputs),
        Executing(Timestamp),
        Blocked(Timestamp),
        Finalizing(Timestamp),
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

    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
    pub struct Port {
        pub id: Uuid,
        pub is_input: bool,
        pub name: Option<String>,
    }

    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Operator {
        pub id: Uuid,
        pub plan_id: Uuid,
        pub name: Option<String>,
        pub ports: Vec<Port>,
        pub state_sequence: Vec<OperatorState>,
    }
}

pub mod plan {
    use super::*;

    /// A Plan edge.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Edge {
        /// The source port id.
        pub source: Uuid,
        /// The target port id.
        pub target: Uuid,
    }

    /// Timestamps of plan state transitions.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct PlanTimestamps {
        /// The time at which the Plan started initialization.
        pub init: Option<Timestamp>,
        /// The time at which the Plan started execution.
        pub executing: Option<Timestamp>,
        /// The time at which Plan execution was completed.
        pub idle: Option<Timestamp>,
        /// The time at which the Plan started cleaning up its resources.
        pub finalizing: Option<Timestamp>,
        /// The time at which the Worker was completely destructed and all resources
        /// were freed.
        pub exit: Option<Timestamp>,
    }

    /// A Query Plan.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Plan {
        /// The ID of this Query Plan.
        pub id: Uuid,
        /// The ID of the Query this Plan is part of.
        pub query_id: Uuid,
        /// The timestamps of various state transitions during the lifetime of this plan.
        pub timestamps: PlanTimestamps,
        /// The optional parent Plan ID. This is useful if an Engine constructs various
        /// types of Plans before execution, sometimes referred to as "lowering".
        /// Examples include a logical and physical plan.
        pub parent_id: Option<Uuid>,
        /// The optional Worker ID of the Worker that executed this Plan.
        ///
        /// If this Plan was not directly executed on a worker, but merely some level
        /// plan in a sequence of lowering stages, then this may be  set to None.
        pub worker_id: Option<Uuid>,
        /// The Operators of this Plan.
        pub operators: Vec<operator::Operator>,
        /// The Edges of this Plan.
        pub edges: Vec<Edge>,
    }
}
