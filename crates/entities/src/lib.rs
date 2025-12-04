//! Type definitions for entities of the model.

use quent_events::Timestamp;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

// n.b. top level options represent missing telemetry

pub mod engine {
    use super::*;

    #[derive(TS, Clone, Default, Deserialize, Serialize)]
    pub struct Engine {
        pub id: Uuid,
        pub init: Option<Timestamp>,
        pub operating: Option<Timestamp>,
        pub finalizing: Option<Timestamp>,
        pub exit: Option<Timestamp>,
    }

    impl Engine {
        pub fn new(engine_id: Uuid) -> Self {
            Self {
                id: engine_id,
                init: None,
                operating: None,
                finalizing: None,
                exit: None,
            }
        }
    }
}
