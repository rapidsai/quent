use serde::{Deserialize, Serialize};

pub mod task {
    use crate::resource::r#use::{Computation, Transfer};

    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Computing {
        pub use_thread: Computation,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Loading {
        pub use_fs_to_mem: Transfer,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Spilling {
        pub use_mem_to_fs: Transfer,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Sending {
        pub use_link: Transfer,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub enum TaskEvent {
        Init,
        Queueing,
        Computing(Computing),
        AllocatingMemory,
        AllocatingStorage,
        Loading(Loading),
        Spilling(Spilling),
        Sending(Sending),
        Exit,
    }
}
