use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryGroupEvent {
    /// The name of this instance of a query group.
    pub instance_name: String,
    /// The id of the engine this query group is executed on.
    pub engine_id: Uuid,
}
