//! Type definitions for entities of the model.

use quent_events::Timestamp;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

// n.b. top level options represent missing telemetry

pub mod engine {
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
    /// Engines accept Queries that they pass to Coordinators which in turn orchestrate their execution.
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Engine {
        /// The ID of this Engine
        pub id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the Engine.
        pub timestamps: EngineTimestamps,
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
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct Coordinator {
        /// The ID of this Coordinator
        pub id: Uuid,
        /// The ID of the Engine this Coordinator was spawned in
        pub engine_id: Uuid,
        /// Timestamps of state transitions throughout the lifetime of the Coordinator.
        pub timestamps: CoordinatorTimestamps,
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
