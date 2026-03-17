use quent_events::{resource::ResourceEvent, trace::TraceEvent};
use quent_query_engine_events::QueryEngineEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod task {

    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Queueing {
        pub operator_id: Uuid,
        pub instance_name: String,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Computing {
        pub use_thread: Uuid,
        /// Working memory for the task (scratch buffers, hash tables, etc.),
        /// separate from the batch's own memory footprint tracked by DataBatch::InHostMemory.
        pub use_host_memory: Uuid,
        pub use_host_memory_bytes: u64,
        /// Nil when not using GPU.
        pub use_gpu_compute: Uuid,
        /// GPU working memory (scratch buffers, intermediate results, etc.),
        /// separate from the batch's own GPU footprint tracked by DataBatch::InGpuMemory.
        /// Nil when not using GPU.
        pub use_gpu_memory: Uuid,
        pub use_gpu_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Allocating {
        pub use_thread: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Loading {
        pub use_thread: Uuid,
        /// Working memory for materialization, separate from the batch's own
        /// memory footprint tracked by DataBatch::InHostMemory.
        pub use_host_memory: Uuid,
        pub use_host_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Spilling {
        pub use_thread: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Sending {
        pub use_thread: Uuid,
        pub use_link: Uuid,
        pub use_link_bytes: u64,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub enum TaskEvent {
        Queueing(Queueing),
        Computing(Computing),
        Allocating(Allocating),
        Loading(Loading),
        Spilling(Spilling),
        Sending(Sending),
        Exit,
    }
}

pub mod data_batch {
    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Init {
        pub operator_id: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InStorage {
        pub use_storage: Uuid,
        pub use_storage_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct LoadingToHostMemory {
        pub use_storage_to_host: Uuid,
        pub use_storage_to_host_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InHostMemory {
        pub use_host_memory: Uuid,
        pub use_host_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct LoadingToGpuMemory {
        pub use_host_mem_to_gpu: Uuid,
        pub use_host_mem_to_gpu_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InGpuMemory {
        pub use_gpu_memory: Uuid,
        pub use_gpu_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct SpillingToHostMemory {
        pub use_gpu_to_host_mem: Uuid,
        pub use_gpu_to_host_mem_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct SpillingToStorage {
        pub use_host_to_storage: Uuid,
        pub use_host_to_storage_bytes: u64,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub enum DataBatchEvent {
        Init(Init),
        InStorage(InStorage),
        LoadingToHostMemory(LoadingToHostMemory),
        InHostMemory(InHostMemory),
        LoadingToGpuMemory(LoadingToGpuMemory),
        InGpuMemory(InGpuMemory),
        SpillingToHostMemory(SpillingToHostMemory),
        SpillingToStorage(SpillingToStorage),
        Exit,
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SimulatorEvent {
    QueryEngineEvent(QueryEngineEvent),
    Task(task::TaskEvent),
    DataBatch(data_batch::DataBatchEvent),
    Resource(ResourceEvent),
    Trace(TraceEvent),
}

impl From<ResourceEvent> for SimulatorEvent {
    fn from(event: ResourceEvent) -> Self {
        SimulatorEvent::Resource(event)
    }
}

impl From<TraceEvent> for SimulatorEvent {
    fn from(event: TraceEvent) -> Self {
        SimulatorEvent::Trace(event)
    }
}
