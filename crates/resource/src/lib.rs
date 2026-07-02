// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The Quent built-in resource constraint.

use quent_schema::{Annotations, Identifier};
use serde::{Deserialize, Serialize};

/// A named, quantified dimension of a resource that usages claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capacity {
    /// The unique name of the capacity within the resource.
    name: Identifier,
    /// The type of capacity.
    kind: CapacityKind,
    /// Whether the capacity is bounded. If all capacities of a resource are
    /// unbounded, then no bounds need to be set, so no bound record type should
    /// exist, and the FSM transition into "operating" shall not have a bounds
    /// argument.
    bounded: bool,
}

impl Capacity {
    pub fn new(name: Identifier, kind: CapacityKind, bounded: bool) -> Self {
        Self {
            name,
            kind,
            bounded,
        }
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn kind(&self) -> CapacityKind {
        self.kind
    }

    pub fn bounded(&self) -> bool {
        self.bounded
    }
}

/// How a capacity relates to the span over which it is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityKind {
    /// A quantity held for the duration of a usage span, e.g. bytes of a
    /// memory.
    Occupancy,
    /// A total quantity processed over a usage span, e.g. bytes sent over a
    /// channel. Dividing it by the span's duration yields the **perceived**
    /// rate.
    Rate,
}

pub type Capacities = indexmap::IndexMap<Identifier, Capacity>;

/// The data a `quent.resource.v1` constraint carries.
///
/// A resource is an entity with one or more capacities that other entities can claim.
///
/// This data is placed on several schema elements. Each variant explains the
/// role of the annotated element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    /// Placed on a resource entity, declaring it a resource providing
    /// `capacities`.
    // Common-case: one capacity.
    Definition(Capacities),
    /// Placed on the record type conveying the bounds of resource `resource`.
    ///
    /// This record is used by a resource's own events.
    Bounds { resource: Identifier },
    /// Placed on the record type conveying a usage of resource `resource`.
    ///
    /// The usage is perceived as held for the duration of the FSM state of the
    /// FSM entity claiming it. This record type can only be used on FSM state
    /// transition events besides exit to ensure the usage is released.
    Usage { resource: Identifier },
}

impl Resource {
    /// The constraint name under which the data is carried.
    pub const NAME: &'static str = "quent.resource.v1";

    /// Deserialize [`Self`] from `annotations`, if it exists.
    pub fn from_annotations(annotations: &Annotations) -> Option<Self> {
        serde_json::from_str(annotations.constraint(Self::NAME)?.data()?).ok()
    }
}

/// A Resource is an Entity with certain Capacities that other Entities may
/// claim through a Usage over some span of time.
///
/// Only states of FSM-type entities provide the guarantee that Usages end
/// (either by transitioning to some next state or the mandatory special exit
/// state which inherently does not hold attributes), thus only FSM-type
/// entities can use resources.
///
/// ## Requirements
///
/// 1. A resource is an entity with at least one [`Capacity`].
/// 2. The [`Identifier`] of a [`Capacity`] is unique within a resource.
/// 3. If and only if any of the resource's [`Capacities`] have a bound, the
///    resource entity has at least one event (the "bounds event") which
///    declares the bounds of all [`Capacities`] that are bounded.
/// 4. An entity can use some quantity of a resource's [`Capacities`] if and
///    only if it is an FSM.
/// 5. The resource named by a usage or bounds is a declared resource.
/// 6. A usage claims only [`Capacities`] declared by its resource.
#[derive(Default)]
pub struct ResourceConstraint;

// TODO(johanpel): validation
