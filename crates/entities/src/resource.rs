use std::collections::HashSet;

use quent_time::TimeUnixNanoSec;
use serde::Serialize;
use smallvec::SmallVec;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{EntityError, Result};

use super::*;

/// The type of capacity of a Resource.
#[derive(TS, Clone, Copy, Debug, Serialize)]
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
#[derive(TS, Clone, Debug, Serialize)]
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
    pub fn unit() -> Self {
        Self {
            name: "unit".into(),
            kind: CapacityKind::Occupancy,
        }
    }
}

/// Declaration of a Resource type
#[derive(TS, Clone, Debug, Serialize)]
pub struct ResourceTypeDecl {
    /// The unique type name for this type of Resource.
    pub name: String,
    // The common case is that a resource has one capacity, don't bother going
    // with HashMap here.
    #[ts(as = "Vec<CapacityDecl>")]
    pub capacities: SmallVec<[CapacityDecl; 1]>,

    /// An unordered set of FSM type names that Use this Resource.
    pub used_by_fsms: HashSet<String>,
}

impl ResourceTypeDecl {
    pub fn new(
        name: impl Into<String>,
        capacities: impl Into<SmallVec<[CapacityDecl; 1]>>,
    ) -> Self {
        Self {
            name: name.into(),
            capacities: capacities.into(),
            used_by_fsms: Default::default(),
        }
    }

    pub fn unit(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            capacities: SmallVec::from([CapacityDecl::unit()]),
            used_by_fsms: Default::default(),
        }
    }

    pub fn capacity(&self, capacity_name: &str) -> Option<&CapacityDecl> {
        self.capacities
            .iter()
            .find(|capacity| capacity.name.eq(capacity_name))
    }

    pub fn try_capacity(&self, capacity_name: &str) -> Result<&CapacityDecl> {
        self.capacity(capacity_name)
            .ok_or(EntityError::InvalidArgument(format!(
                "unknown capacity \"{capacity_name}\" for resource type {}. Must be one of: {:?}",
                self.name,
                self.capacities
                    .iter()
                    .map(|capacity| capacity.name.as_str())
                    .collect::<Vec<_>>()
            )))
    }
}

/// A Resource Capacity
#[derive(TS, Clone, Debug, Serialize, PartialEq)]
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
#[derive(TS, Clone, Debug, Default, Serialize)]
pub struct ResourceOperatingState {
    pub timestamp: TimeUnixNanoSec,
    pub capacities: Vec<CapacityValue>,
}

/// Resource states.
#[derive(TS, Clone, Debug, Serialize)]
pub enum ResourceState {
    Init(TimeUnixNanoSec),
    Operating(ResourceOperatingState),
    Resizing(TimeUnixNanoSec),
    Finalizing(TimeUnixNanoSec),
    Exit(TimeUnixNanoSec),
}

/// A Resource of which its Capacities cannot be resized.
#[derive(TS, Clone, Default, Debug, Serialize)]
pub struct Resource {
    /// The ID of this Resource.
    pub id: Uuid,
    /// The name of this Resource.
    pub instance_name: Option<String>,
    /// The type of this Resource.
    pub type_name: String,
    /// The scope of this Resource.
    pub scope: Option<EntityRef>,
    /// The sequence of states that this Resource
    #[serde(skip)]
    #[ts(skip)]
    pub state_sequence: Vec<ResourceState>,
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
#[derive(TS, Clone, Debug, Default, Serialize)]
pub struct ResourceGroupTimestamps {
    /// The time at which the ResourceGroup started initialization.
    pub init: Option<TimeUnixNanoSec>,
    /// The time at which the ResourceGroup started operating.
    pub operating: Option<TimeUnixNanoSec>,
    /// The time at which the ResourceGroup started shutting down and
    /// cleaning up its resources.
    pub finalizing: Option<TimeUnixNanoSec>,
    /// The time at which the ResourceGroup was completely destructed and
    /// all resources were freed.
    pub exit: Option<TimeUnixNanoSec>,
}

/// A Group of Resources.
#[derive(TS, Clone, Debug, Default, Serialize)]
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
    #[serde(skip)]
    #[ts(skip)]
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

#[derive(TS, Clone, Debug, PartialEq, Serialize)]
pub struct Use {
    pub resource: Uuid,
    #[ts(as = "Vec<CapacityDecl>")]
    pub capacities: SmallVec<[CapacityValue; 1]>,
}

impl Use {
    pub fn new(resource: Uuid, capacities: impl Into<SmallVec<[CapacityValue; 1]>>) -> Self {
        Self {
            resource,
            capacities: capacities.into(),
        }
    }

    pub fn unit(resource: Uuid) -> Self {
        Self {
            resource,
            capacities: SmallVec::from([CapacityValue::new("unit", 1)]),
        }
    }
}

impl Related for Use {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        [EntityRef::Resource(self.resource)].into_iter()
    }
}
