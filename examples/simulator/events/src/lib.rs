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
        pub use_memory: Uuid,
        pub use_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Allocating {
        pub use_thread: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Loading {
        pub use_thread: Uuid,
        pub use_fs_to_mem: Uuid,
        pub use_fs_to_mem_bytes: u64,
        pub use_memory: Uuid,
        pub use_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Spilling {
        pub use_thread: Uuid,
        pub use_mem_to_fs: Uuid,
        pub use_mem_to_fs_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Sending {
        pub use_thread: Uuid,
        pub use_link: Uuid,
        pub use_link_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct GpuComputing {
        pub use_thread: Uuid,
        pub use_gpu_compute: Uuid,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub enum TaskEvent {
        Queueing(Queueing),
        Computing(Computing),
        Allocating(Allocating),
        Loading(Loading),
        Spilling(Spilling),
        Sending(Sending),
        GpuComputing(GpuComputing),
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
        pub use_filesystem: Uuid,
        pub use_filesystem_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct LoadingToHostMemory {
        pub use_fs_to_mem: Uuid,
        pub use_fs_to_mem_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InHostMemory {
        pub use_memory: Uuid,
        pub use_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct LoadingToGpuMemory {
        pub use_mem_to_gpu: Uuid,
        pub use_mem_to_gpu_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct InGpuMemory {
        pub use_gpu_memory: Uuid,
        pub use_gpu_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct SpillingToHostMemory {
        pub use_gpu_to_mem: Uuid,
        pub use_gpu_to_mem_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct SpillingToStorage {
        pub use_mem_to_fs: Uuid,
        pub use_mem_to_fs_bytes: u64,
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
