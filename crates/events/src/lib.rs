//! Type definitions of entity events.

use py_rs::PY;
use quent_time::{Timestamp, timestamp};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[cfg(feature = "q")]
pub mod q;

#[derive(Debug, Deserialize, Serialize)]
pub struct Event<T> {
    pub id: Uuid,
    pub timestamp: Timestamp,
    pub data: T,
}

impl<T> Event<T> {
    #[inline]
    pub fn new_now(id: Uuid, data: T) -> Self {
        Self {
            id,
            timestamp: timestamp(),
            data,
        }
    }

    pub fn new(id: Uuid, timestamp: Timestamp, data: T) -> Self {
        Self {
            id,
            timestamp,
            data,
        }
    }
}

pub mod engine {
    use quent_attributes::Attribute;

    use super::*;

    /// Attributes describing details about the implementation of this Engine
    #[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
    pub struct EngineImplementationAttributes {
        /// The name of this Engine implementation, e.g. "SiriusDB", "Velox", "DataFusion", etc.
        pub name: Option<String>,
        /// The version of this Engine implementation, e.g. "13.3.7"
        pub version: Option<String>,
        /// Arbitrary attributes defined at run time.
        pub custom_attributes: Vec<Attribute>,
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

pub mod query_group {
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
    pub enum QueryGroupEvent {
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
        pub query_group_id: Uuid,
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

    /// A directed edge of a Plan DAG.
    // TODO(johanpel): should we flatten this?
    #[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
    pub struct Edge {
        /// The ID of the port sourcing data.
        pub source: Uuid,
        /// The ID of the port sinking data.
        pub target: Uuid,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Init {
        pub name: String,
        pub query_id: Uuid,
        pub edges: Vec<Edge>,

        pub worker_id: Option<Uuid>,
        pub parent_plan_id: Option<Uuid>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Executing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Idle {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum PlanEvent {
        Init(Init),
        Executing(Executing),
        Idle(Idle),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod operator {
    use super::*;

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Port {
        pub id: Uuid,
        pub name: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Init {
        pub plan_id: Uuid,
        pub parent_operator_ids: Vec<Uuid>,
        pub name: Option<String>,
        pub ports: Vec<Port>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct WaitingForInputs {
        pub ports: Vec<Uuid>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Executing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Blocked {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
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

pub mod resource {
    use super::*;

    /// The scope of a Resource.
    ///
    /// This represents the scope in which a Resource is shared. Only constructs
    /// owned by this Scope can have a Use of this Resource.
    #[derive(TS, Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
    pub enum Scope {
        Engine(Uuid),
        QueryGroup(Uuid),
        Query(Uuid),
        Plan(Uuid),
        Worker(Uuid),
        Operator(Uuid),
        Port(Uuid),
        ResourceGroup(Uuid),
    }

    impl From<Scope> for Uuid {
        fn from(value: Scope) -> Self {
            match value {
                Scope::Engine(uuid) => uuid,
                Scope::QueryGroup(uuid) => uuid,
                Scope::Query(uuid) => uuid,
                Scope::Plan(uuid) => uuid,
                Scope::Worker(uuid) => uuid,
                Scope::Operator(uuid) => uuid,
                Scope::Port(uuid) => uuid,
                Scope::ResourceGroup(uuid) => uuid,
            }
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Resource {
        pub instance_name: String,
        pub type_name: String, // TODO(johanpel): for now solve this like so, but this could be generated code too
        pub scope: Scope,
    }

    pub mod memory {
        use super::*;

        #[derive(Debug, Deserialize, Serialize)]
        pub struct Init {
            pub resource: Resource,
        }

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Operating {
            pub capacity_bytes: u64,
        }

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Resizing {
            pub requested_bytes: u64,
        }

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Finalizing {
            pub unreclaimed_bytes: u64,
        }

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Exit {}

        #[derive(Debug, Deserialize, Serialize)]
        pub enum MemoryEvent {
            Init(Init),
            Operating(Operating),
            Resizing(Resizing),
            Finalizing(Finalizing),
            Exit(Exit),
        }
    }

    pub mod processor {

        use super::*;

        #[derive(Debug, Deserialize, Serialize)]
        pub struct Init {
            pub resource: Resource,
        }

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Operating {}

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Finalizing {}

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Exit {}

        #[derive(Debug, Deserialize, Serialize)]
        pub enum ProcessorEvent {
            Init(Init),
            Operating(Operating),
            Finalizing(Finalizing),
            Exit(Exit),
        }
    }

    pub mod channel {
        use super::*;

        #[derive(Debug, Deserialize, Serialize)]
        pub struct Init {
            pub resource: Resource,
            pub source_id: Uuid,
            pub target_id: Uuid,
        }

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Operating {}

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Finalizing {}

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Exit {}

        #[derive(Debug, Deserialize, Serialize)]
        pub enum ChannelEvent {
            Init(Init),
            Operating(Operating),
            Finalizing(Finalizing),
            Exit(Exit),
        }
    }

    pub mod group {
        use super::*;

        #[derive(Debug, Deserialize, Serialize)]
        pub struct Init {
            pub resource: Resource,
        }

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Operating {}

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Finalizing {}

        #[derive(Debug, Default, Deserialize, Serialize)]
        pub struct Exit {}

        #[derive(Debug, Deserialize, Serialize)]
        pub enum ResourceGroupEvent {
            Init(Init),
            Operating(Operating),
            Finalizing(Finalizing),
            Exit(Exit),
        }
    }

    use crate::resource::{channel::ChannelEvent, memory::MemoryEvent, processor::ProcessorEvent};

    #[derive(Debug, Deserialize, Serialize)]
    pub enum ResourceEvent {
        Memory(MemoryEvent),
        Processor(ProcessorEvent),
        Channel(ChannelEvent),
    }

    pub mod r#use {
        use super::*;

        #[derive(Debug, Deserialize, Serialize)]
        pub struct Allocation {
            pub resource_id: Uuid,
            pub used_bytes: u64,
        }

        #[derive(Debug, Deserialize, Serialize)]
        pub struct Transfer {
            pub resource_id: Uuid,
            pub used_bytes: u64,
        }

        #[derive(Debug, Deserialize, Serialize)]
        pub struct Computation {
            pub resource_id: Uuid,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum EventData {
    Engine(engine::EngineEvent),
    QueryGroup(query_group::QueryGroupEvent),
    Worker(worker::WorkerEvent),
    Query(query::QueryEvent),
    Plan(plan::PlanEvent),
    Operator(operator::OperatorEvent),
    Resource(resource::ResourceEvent),
    ResourceGroup(resource::group::ResourceGroupEvent),
    #[cfg(feature = "q")]
    Q(q::QEvent),
}

impl From<engine::EngineEvent> for EventData {
    fn from(value: engine::EngineEvent) -> Self {
        Self::Engine(value)
    }
}
impl From<query_group::QueryGroupEvent> for EventData {
    fn from(value: query_group::QueryGroupEvent) -> Self {
        Self::QueryGroup(value)
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
impl From<resource::group::ResourceGroupEvent> for EventData {
    fn from(value: resource::group::ResourceGroupEvent) -> Self {
        Self::ResourceGroup(value)
    }
}

impl From<resource::ResourceEvent> for EventData {
    fn from(value: resource::ResourceEvent) -> Self {
        Self::Resource(value)
    }
}
impl From<resource::memory::MemoryEvent> for EventData {
    fn from(value: resource::memory::MemoryEvent) -> Self {
        Self::Resource(resource::ResourceEvent::Memory(value))
    }
}
impl From<resource::channel::ChannelEvent> for EventData {
    fn from(value: resource::channel::ChannelEvent) -> Self {
        Self::Resource(resource::ResourceEvent::Channel(value))
    }
}
impl From<resource::processor::ProcessorEvent> for EventData {
    fn from(value: resource::processor::ProcessorEvent) -> Self {
        Self::Resource(resource::ResourceEvent::Processor(value))
    }
}

#[cfg(feature = "q")]
impl From<q::QEvent> for EventData {
    fn from(value: q::QEvent) -> Self {
        Self::Q(value)
    }
}
#[cfg(feature = "q")]
impl From<q::task::TaskEvent> for EventData {
    fn from(value: q::task::TaskEvent) -> Self {
        Self::Q(q::QEvent::Task(value))
    }
}
#[cfg(feature = "q")]
impl From<q::record_batch::RecordBatchEvent> for EventData {
    fn from(value: q::record_batch::RecordBatchEvent) -> Self {
        Self::Q(q::QEvent::RecordBatch(value))
    }
}
