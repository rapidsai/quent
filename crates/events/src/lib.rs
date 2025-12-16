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

/// Support for custom attributes defined at run-time.
pub mod attributes {
    use super::*;

    /// A group of [`Attribute`]s.
    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
    pub struct Struct(Vec<Attribute>);

    /// A sequence of [`Value`]s.
    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
    #[ts(untagged)]
    pub enum List {
        U8(Vec<u8>),
        U16(Vec<u16>),
        U32(Vec<u32>),
        U64(Vec<u64>),
        I8(Vec<i8>),
        I16(Vec<i16>),
        I32(Vec<i32>),
        I64(Vec<i64>),
        F32(Vec<f32>),
        F64(Vec<f64>),
        String(Vec<String>),
        Struct(Vec<Struct>),
    }

    /// An [`Attribute`] value.
    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
    #[ts(untagged)]
    pub enum Value {
        U8(u8),
        U16(u16),
        U32(u32),
        U64(u64),
        I8(u8),
        I16(i16),
        I32(i32),
        I64(i64),
        F32(f32),
        F64(f64),
        String(String),
        Struct(Struct),
        List(List),
    }

    /// A key-value pair.
    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
    pub struct Attribute {
        pub key: String,
        pub value: Value,
    }
}

pub mod engine {
    use crate::attributes::Attribute;

    use super::*;

    /// Attributes describing details about the implementation of this Engine
    #[derive(TS, Clone, Debug, Default, Deserialize, Serialize)]
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
    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
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

    #[derive(TS, Clone, Debug, Deserialize, Serialize)]
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

    #[derive(Debug, Deserialize, Serialize)]
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

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Resource {
        pub name: String,
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
        pub enum MemoryResourceEvent {
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
        pub enum ProcessorResourceEvent {
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
        pub enum ChannelResourceEvent {
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

    use crate::resource::{
        channel::ChannelResourceEvent, memory::MemoryResourceEvent,
        processor::ProcessorResourceEvent,
    };

    #[derive(Debug, Deserialize, Serialize)]
    pub enum ResourceEvent {
        Memory(MemoryResourceEvent),
        Processor(ProcessorResourceEvent),
        Channel(ChannelResourceEvent),
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
            pub transferred_bytes: u64,
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
impl From<resource::memory::MemoryResourceEvent> for EventData {
    fn from(value: resource::memory::MemoryResourceEvent) -> Self {
        Self::Resource(resource::ResourceEvent::Memory(value))
    }
}
impl From<resource::channel::ChannelResourceEvent> for EventData {
    fn from(value: resource::channel::ChannelResourceEvent) -> Self {
        Self::Resource(resource::ResourceEvent::Channel(value))
    }
}
impl From<resource::processor::ProcessorResourceEvent> for EventData {
    fn from(value: resource::processor::ProcessorResourceEvent) -> Self {
        Self::Resource(resource::ResourceEvent::Processor(value))
    }
}
