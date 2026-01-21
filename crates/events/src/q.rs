use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod task {

    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Initializing {
        pub operator_id: Uuid,
        pub name: Option<String>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Queueing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Computing {
        pub use_task_thread: Uuid,
        pub use_main_memory: Uuid,
        pub use_main_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct AllocatingMemory {
        pub use_task_thread: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Loading {
        pub use_task_thread: Uuid,
        pub use_fs_to_mem: Uuid,
        pub use_fs_to_mem_bytes: u64,
        pub use_main_memory: Uuid,
        pub use_main_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct AllocatingStorage {
        pub use_task_thread: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Spilling {
        pub use_task_thread: Uuid,
        pub use_mem_to_fs: Uuid,
        pub use_mem_to_fs_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Sending {
        pub use_task_thread: Uuid,
        pub use_link: Uuid,
        pub use_link_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum TaskEvent {
        Initializing(Initializing),
        Queueing(Queueing),
        Computing(Computing),
        AllocatingMemory(AllocatingMemory),
        Loading(Loading),
        AllocatingStorage(AllocatingStorage),
        Spilling(Spilling),
        Sending(Sending),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod record_batch {
    use super::*;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Initializing {
        pub operator_id: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Idle {
        pub use_filesystem: Option<Uuid>,
        pub use_filesystem_bytes: u64,
        pub use_main_memory: Option<Uuid>,
        pub use_main_memory_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Moving {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum RecordBatchEvent {
        Initializing(Initializing),
        Idle(Idle),
        Moving(Moving),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum QEvent {
    Task(task::TaskEvent),
    RecordBatch(record_batch::RecordBatchEvent),
}
