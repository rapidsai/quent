use quent_attributes::Attribute;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct Declaration {
    /// The ID of the operator this port belongs to.
    pub operator_id: Uuid,
    /// The name of this port.
    pub instance_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Statistics {
    pub custom_attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PortEvent {
    Declaration(Declaration),
    Statistics(Statistics),
}
