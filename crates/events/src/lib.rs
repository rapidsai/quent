use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type Timestamp = u64;

pub mod engine {

    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Init {
        pub id: Uuid,
        pub t: Timestamp,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Operating {
        id: Uuid,
        t: Timestamp,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Finalizing {
        id: Uuid,
        t: Timestamp,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Done {
        id: Uuid,
        t: Timestamp,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub enum Event {
        Init(Init),
        Operating(Operating),
        Finalizing(Finalizing),
        Done(Done),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Event {
    Engine(engine::Event),
    Flush,
}
