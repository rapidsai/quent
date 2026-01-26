use std::collections::HashSet;

use quent_time::TimeUnixNanoSec;
use serde::Serialize;
use smallvec::SmallVec;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    error::{EntityError, Result},
    fsm::{Fsm, State, StateSequenceBuilder},
};

use super::*;

/// The type of capacity of a Resource.
#[derive(TS, Clone, Copy, Debug, Serialize)]
pub enum CapacityType {
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
    pub kind: CapacityType,
}

impl CapacityDecl {
    pub fn new_occupancy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: CapacityType::Occupancy,
        }
    }
    pub fn new_rate(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: CapacityType::Rate,
        }
    }
    pub fn unit() -> Self {
        Self {
            name: "unit".into(),
            kind: CapacityType::Occupancy,
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

impl State for ResourceState {
    fn name(&self) -> &str {
        match self {
            ResourceState::Init(_) => "init",
            ResourceState::Operating(_) => "operating",
            ResourceState::Resizing(_) => "resizing",
            ResourceState::Finalizing(_) => "finalizing",
            ResourceState::Exit(_) => "exit",
        }
    }
    fn uses(&self) -> impl Iterator<Item = &Use> {
        std::iter::empty()
    }
    fn timestamp(&self) -> TimeUnixNanoSec {
        *match self {
            ResourceState::Init(ts) => ts,
            ResourceState::Operating(s) => &s.timestamp,
            ResourceState::Resizing(ts) => ts,
            ResourceState::Finalizing(ts) => ts,
            ResourceState::Exit(ts) => ts,
        }
    }
    fn attributes(&self) -> impl Iterator<Item = &quent_attributes::Attribute> {
        std::iter::empty()
    }
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        std::iter::empty()
    }
}

#[derive(Default)]
pub struct ResourceBuilder {
    id: Uuid,
    instance_name: Option<String>,
    type_name: String,
    scope: Option<EntityRef>,
    state_sequence: StateSequenceBuilder<ResourceState>,
}

impl IncompleteEntity for ResourceBuilder {
    fn new(id: Uuid) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
}

impl ResourceBuilder {
    pub fn type_name(&self) -> &str {
        &self.type_name
    }
    pub fn set_type_name(&mut self, type_name: impl Into<String>) {
        self.type_name = type_name.into();
    }
    pub fn set_instance_name(&mut self, instance_name: Option<String>) {
        self.instance_name = instance_name;
    }
    pub fn set_scope(&mut self, scope: EntityRef) {
        self.scope = Some(scope);
    }
    pub fn push_state(&mut self, state: ResourceState) {
        self.state_sequence.push_state(state);
    }
    pub fn try_build(self) -> Resource {
        Resource {
            id: self.id,
            instance_name: self.instance_name,
            type_name: self.type_name,
            scope: self.scope,
            state_sequence: self.state_sequence.sequence,
        }
    }
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
    /// The sequence of states that this resource went through.
    #[serde(skip)]
    #[ts(skip)]
    pub state_sequence: Vec<ResourceState>,
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

impl Fsm for Resource {
    type State = ResourceState;
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
    fn instance_name(&self) -> Option<&str> {
        self.instance_name.as_deref()
    }
    fn len(&self) -> usize {
        self.state_sequence.len()
    }
    fn index(&self, index: usize) -> Option<&Self::State> {
        self.state_sequence.get(index)
    }
    fn states(&self) -> impl ExactSizeIterator<Item = &Self::State> {
        self.state_sequence.iter()
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

impl IncompleteEntity for ResourceGroup {
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
