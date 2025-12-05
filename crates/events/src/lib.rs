//! Type definitions of entity events.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub type Timestamp = u64;

#[inline]
fn timestamp() -> Timestamp {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;
    // Narrowing conversion to u64 limits this to Unix timestamp in seconds: 18446744073709551617
    // Which is in the 26th century
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_nanos() as u64)
        .unwrap_or_default()
    // TODO(johanpel): consider to do something else instead of unwrap_or_default, perhaps using Instant as described in the duration_since docs.
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Event<T> {
    pub id: Uuid,
    pub timestamp: Timestamp,
    pub data: T,
}

impl<T> Event<T> {
    #[inline]
    pub fn new(id: Uuid, data: T) -> Self {
        Self {
            id,
            timestamp: timestamp(),
            data,
        }
    }
}

pub mod engine {
    use super::*;

    /// Attributes describing details about the implementation of this Engine
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct EngineImplementationAttributes {
        /// The name of this Engine implementation, e.g. "SiriusDB", "Velox", "DataFusion", etc.
        pub name: Option<String>,
        /// The version of this Engine implementation, e.g. "13.3.7"
        pub version: Option<String>,
        // TODO(johanpel): much more
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Init {
        pub implementation: Option<EngineImplementationAttributes>,
        pub name: Option<String>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Operating {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum EngineEvent {
        Init(Init),
        Operating(Operating),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod coordinator {
    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Init {
        pub engine_id: Uuid,
        pub name: Option<String>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Operating {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum CoordinatorEvent {
        Init(Init),
        Operating(Operating),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod worker {
    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Init {
        pub engine_id: Uuid,
        pub name: Option<String>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Operating {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum WorkerEvent {
        Init(Init),
        Operating(Operating),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod query {
    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Init {
        pub coordinator_id: Uuid,
        pub name: Option<String>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Planning {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Executing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Idle {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum QueryEvent {
        Init(Init),
        Planning(Planning),
        Executing(Executing),
        Idle(Idle),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod plan {
    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Init {
        pub query_id: Uuid,
        pub worker_id: Option<Uuid>,
        pub parent_id: Option<Uuid>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Executing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Idle {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum PlanEvent {
        Init(Init),
        Executing(Executing),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod operator {
    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Init {
        pub plan_id: Uuid,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct WaitingForInputs {
        pub ports: Vec<Uuid>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Executing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Blocked {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum OperatorEvent {
        Init(Init),
        WaitingForInputs(WaitingForInputs),
        Executing(Executing),
        Blocked(Blocked),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum EventData {
    Engine(engine::EngineEvent),
    Coordinator(coordinator::CoordinatorEvent),
    Worker(worker::WorkerEvent),
    Query(query::QueryEvent),
    Plan(plan::PlanEvent),
    Operator(operator::OperatorEvent),
}

impl From<engine::EngineEvent> for EventData {
    fn from(value: engine::EngineEvent) -> Self {
        Self::Engine(value)
    }
}
impl From<coordinator::CoordinatorEvent> for EventData {
    fn from(value: coordinator::CoordinatorEvent) -> Self {
        Self::Coordinator(value)
    }
}
impl From<worker::WorkerEvent> for EventData {
    fn from(value: worker::WorkerEvent) -> Self {
        Self::Worker(value)
    }
}
impl From<query::QueryEvent> for EventData {
    fn from(value: query::QueryEvent) -> Self {
        Self::Query(value)
    }
}
impl From<plan::PlanEvent> for EventData {
    fn from(value: plan::PlanEvent) -> Self {
        Self::Plan(value)
    }
}
impl From<operator::OperatorEvent> for EventData {
    fn from(value: operator::OperatorEvent) -> Self {
        Self::Operator(value)
    }
}
