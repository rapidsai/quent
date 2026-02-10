use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Init {
    pub query_group_id: Uuid,
    pub instance_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum QueryEvent {
    Init(Init),
    Planning,
    Executing,
    Exit,
}
