use std::collections::HashMap;

use py_rs::PY;
use quent_time::Timestamp;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{EntityError, Result};

use super::*;

/// The type of capacity of a Resource.
#[derive(TS, PY, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum CapacityKind {
    /// The Use value represents the amount of Resource Capacity
    /// held/occupied during a Span.
    Occupancy,
    /// The Use value represents the total quantity of work performed over
    /// the span.
    ///
    /// It does NOT represent the rate itself, as this can be derived by
    /// dividing it over the span which is already captured by the time
    /// stamps of events.
    ///
    /// # Example:
    ///
    /// Consider 100 bytes transferred over a 50 second network connection.
    /// Use value should be 100. This is the total number of bytes, not the
    /// rate/sec. The average rate over the span is computed as:
    /// value / span.duration() = 2 bytes/sec.
    Rate,
}

/// Declaration of a Capacity of a Resource
#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct CapacityDecl {
    pub name: String,
    pub kind: CapacityKind,
}

impl CapacityDecl {
    pub fn new_occupancy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: CapacityKind::Occupancy,
        }
    }
    pub fn new_rate(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: CapacityKind::Rate,
        }
    }
}

/// A Resource Capacity
#[derive(TS, PY, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CapacityValue {
    // TODO(johanpel): consider making this a small index into whats declared at init
    pub name: String,
    pub value: Option<u64>,
}

impl CapacityValue {
    pub fn new(name: impl Into<String>, value: u64) -> Self {
        Self {
            name: name.into(),
            value: Some(value),
        }
    }
    pub fn new_null(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: None,
        }
    }
}

/// Attributes of the "Operating" state of a Resource.
#[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResourceOperatingState {
    pub timestamp: Timestamp,
    pub capacities: Vec<CapacityValue>,
}

/// Resource states.
#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub enum ResourceState {
    Init(Timestamp),
    Operating(ResourceOperatingState),
    Resizing(Timestamp),
    Finalizing(Timestamp),
    Exit(Timestamp),
}

/// A Resource of which its Capacities cannot be resized.
#[derive(TS, PY, Clone, Default, Debug, Deserialize, Serialize)]
pub struct Resource {
    /// The ID of this Resource
    pub id: Uuid,
    /// The name of this Resource
    pub instance_name: Option<String>,
    /// The name of this Resource type
    pub type_name: String,
    /// The scope of this Resource.
    pub scope: Option<EntityRef>,
    /// The Capacities of this Resource.
    ///
    /// If this is empty, this is Unit resource.
    pub capacities: HashMap<String, CapacityDecl>,
    /// The sequence of states that this Resource
    pub state_sequence: Vec<ResourceState>,
}

impl Resource {
    pub fn capacity(&self, capacity_name: &str) -> Result<&CapacityDecl> {
        self.capacities
            .get(capacity_name)
            .ok_or(EntityError::InvalidArgument(format!(
                "unknown capacity \"{capacity_name}\" for resource {}. Must be one of: {:?}",
                self.id,
                self.capacities.keys()
            )))
    }
}

impl Entity for Resource {
    fn new(id: Uuid) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Related for Resource {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        if let Some(scope) = self.scope {
            vec![scope].into_iter()
        } else {
            // Shouldn't happen after filtering out parentless things, but
            // for good measure:
            vec![].into_iter()
        }
    }
}

/// Timestamps (nanoseconds since Unix epoch) of state transitions of a
/// ResourceGroup.
#[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResourceGroupTimestamps {
    /// The time at which the ResourceGroup started initialization.
    pub init: Option<Timestamp>,
    /// The time at which the ResourceGroup started operating.
    pub operating: Option<Timestamp>,
    /// The time at which the ResourceGroup started shutting down and
    /// cleaning up its resources.
    pub finalizing: Option<Timestamp>,
    /// The time at which the ResourceGroup was completely destructed and
    /// all resources were freed.
    pub exit: Option<Timestamp>,
}

/// A Group of Resources.
#[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResourceGroup {
    /// The ID of this Resource Group
    pub id: Uuid,
    /// The name of the type of this Resource Group
    pub type_name: Option<String>,
    /// The name of the instance of this Resource Group
    pub instance_name: Option<String>,
    /// The scope of this Resource Group
    pub scope: Option<EntityRef>,
    /// The timestamps of state transitions of this ResourceGroup.
    pub timestamps: ResourceGroupTimestamps,
}

impl Entity for ResourceGroup {
    fn new(id: Uuid) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Related for ResourceGroup {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        if let Some(scope) = self.scope {
            vec![scope].into_iter()
        } else {
            // Shouldn't happen after filtering out parentless things, but
            // for good measure:
            vec![].into_iter()
        }
    }
}

#[derive(TS, PY, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Use {
    pub resource: Uuid,
    // TODO(johanpel): consider using an index into a list of capacity names
    // in a resource vs. an attribute key as string
    pub capacities: Vec<CapacityValue>,
}

impl Use {
    pub fn new(resource: Uuid, capacities: Vec<CapacityValue>) -> Self {
        Self {
            resource,
            capacities,
        }
    }

    pub fn unit(resource: Uuid) -> Self {
        Self {
            resource,
            // TODO(johanpel): figure out how to best deal with unit
            // resource capacity (also in analysis)
            capacities: vec![],
        }
    }
}

impl Related for Use {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        [EntityRef::Resource(self.resource)].into_iter()
    }
}
