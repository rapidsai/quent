use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Init {
    pub parent_engine_id: Uuid,
    pub instance_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum WorkerEvent {
    Init(Init),
    Exit,
}
