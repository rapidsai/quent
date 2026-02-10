use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct PortEvent {
    /// The ID of the operator this port belongs to.
    pub operator_id: Uuid,
    /// The name of this port.
    pub instance_name: String,
}
