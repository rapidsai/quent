// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Traits, types and functions for Resource and Resource Groups

use std::collections::HashSet;

use quent_time::{SpanNanoSec, span::SpanUnixNanoSec};
use smallvec::SmallVec;
use uuid::Uuid;

use super::*;

pub mod collection;
pub mod runtime;
pub mod tree;

/// Trait for types that are considered a [`Resource`].
pub trait Resource: Entity {
    /// The id of the parent resource group.
    fn parent_group_id(&self) -> Uuid;
}

/// Trait for types under which [`Resource`]s can be grouped.
pub trait ResourceGroup: Entity {
    /// The parent of this Resource Group.
    ///
    /// If this is None, it is considered the root of the global application's
    /// resource tree.
    fn parent_group_id(&self) -> Option<Uuid>;

    /// Convenience function to create a type decl from this resource group.
    fn resource_group_type_decl(
        &self,
        used_by_entity_types: HashSet<String>,
        contains_resource_types: HashSet<String>,
    ) -> ResourceGroupTypeDecl {
        ResourceGroupTypeDecl {
            name: self.type_name().to_owned(),
            used_by_entity_types,
            contains_resource_types,
        }
    }
}

/// Trait for types that represent the [`Usage`] of a [`Resource`].
pub trait Usage<'a> {
    /// Return the ID of the entity using the resource.
    fn entity_id(&self) -> Uuid;
    /// Return the ID of the resource being used.
    fn resource_id(&self) -> Uuid;
    /// Return an iterator over the amount of usage for each of the capacities of the resource.
    fn capacities(&self) -> impl Iterator<Item = &'a CapacityValue>;
    /// Return the span of time of the usage.
    fn span(&self) -> SpanUnixNanoSec;
}

/// Trait for types that can hold multiple [`Usage`]s.
pub trait Using {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>>;
}

/// The type of capacity of a Resource.
#[derive(Clone, Copy, Debug)]
pub enum CapacityType {
    /// The Usage value represents the amount of Resource Capacity
    /// held/occupied during a Span.
    Occupancy,
    /// The Usage value represents the total quantity of work performed over
    /// the span.
    ///
    /// It does NOT represent the rate itself, as this can be derived by
    /// dividing it over the span which is already captured by the time
    /// stamps of events.
    ///
    /// # Example:
    ///
    /// Consider 100 bytes transferred over a 50 second network connection.
    /// Usage value should be 100. This is the total number of bytes, not the
    /// rate/sec. The average rate over the span is computed as:
    /// value / span.duration() = 2 bytes/sec.
    Rate,
    // TODO(johanpel): the rate capacity type may need an additional variant
    // that defines whether the thing providing the rate operates synchronously
    // or asynchronously. If it's the latter, we can only show perceived
    // throughputs/durations.
}

impl CapacityType {
    /// Interpret a value from a [`CapacityValue`] based on this [`CapacityType`].
    pub fn reinterpret_capacity_value(&self, value: u64, span: SpanNanoSec) -> f64 {
        match self {
            CapacityType::Occupancy => value as f64,
            CapacityType::Rate => value as f64 / span.duration() as f64,
        }
    }
}

/// Declaration of a capacity of a [`Resource`].
#[derive(Clone, Debug)]
pub struct CapacityDecl {
    /// The name of the capacity.
    pub name: String,
    /// The kind of capacity.
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

/// Declaration of a [`Resource`] type
#[derive(Clone, Debug)]
pub struct ResourceTypeDecl {
    /// The unique type name for this type of Resource.
    pub name: String,
    // The common case is that a resource has one capacity, don't bother going
    // with HashMap here.
    pub capacities: SmallVec<[CapacityDecl; 1]>,
    /// The unique names of the entities that used this resource.
    pub used_by: HashSet<String>,
}

impl ResourceTypeDecl {
    pub fn new(
        name: impl Into<String>,
        capacities: impl Into<SmallVec<[CapacityDecl; 1]>>,
    ) -> Self {
        Self {
            name: name.into(),
            capacities: capacities.into(),
            used_by: Default::default(),
        }
    }

    pub fn unit(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            capacities: SmallVec::from([CapacityDecl::unit()]),
            used_by: Default::default(),
        }
    }

    pub fn capacity(&self, capacity_name: &str) -> Option<&CapacityDecl> {
        self.capacities
            .iter()
            .find(|capacity| capacity.name.eq(capacity_name))
    }

    pub fn try_capacity(&self, capacity_name: &str) -> AnalyzerResult<&CapacityDecl> {
        self.capacity(capacity_name)
            .ok_or(AnalyzerError::InvalidArgument(format!(
                "unknown capacity \"{capacity_name}\" for resource type {}. Must be one of: {:?}",
                self.name,
                self.capacities
                    .iter()
                    .map(|capacity| capacity.name.as_str())
                    .collect::<Vec<_>>()
            )))
    }
}

/// Declaration of a [`ResourceGroup`] type.
#[derive(Clone, Debug)]
pub struct ResourceGroupTypeDecl {
    /// The unique type name for this type of Resource.
    pub name: String,

    /// The type names of the entities that used Resources in this group.
    pub used_by_entity_types: HashSet<String>,
    /// The type names of the resources that are in this group.
    pub contains_resource_types: HashSet<String>,
}

/// A value related to the capacity of a [`Resource`].
#[derive(Clone, Debug, PartialEq)]
pub struct CapacityValue {
    pub name: &'static str,
    pub value: Option<u64>,
}

impl CapacityValue {
    pub fn new(name: &'static str, value: u64) -> Self {
        Self {
            name,
            value: Some(value),
        }
    }
    pub fn new_null(name: &'static str) -> Self {
        Self { name, value: None }
    }
}

/// Attributes of the "Operating" state of a Resource.
// TODO(johanpel): consider SVO
#[derive(Debug)]
pub struct ResourceCapacities(pub Vec<CapacityValue>);
