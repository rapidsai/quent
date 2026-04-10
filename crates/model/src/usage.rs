// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resource usage type linking FSM states to resources.

use crate::Ref;
use crate::Resource;

/// A usage of a resource, linking an FSM state to a specific resource instance
/// and its claimed capacity.
///
/// `Usage<T>` requires `T: Resource`. The `capacity` field type is determined
/// by the resource's associated `CapacityValue` type:
///
/// - For a `Memory` resource: `CapacityValue` contains `used_bytes: u64`
/// - For a `Processor` (unit resource): `CapacityValue` is `()`
/// - For custom resources: `CapacityValue` matches the operating state fields
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "T::CapacityValue: serde::Serialize",
        deserialize = "T::CapacityValue: serde::de::DeserializeOwned"
    ))
)]
pub struct Usage<T: Resource> {
    /// Typed reference to the resource instance being used.
    pub resource_id: Ref<T>,
    /// The capacity claimed from the resource.
    pub capacity: T::CapacityValue,
}

impl<T: Resource> Clone for Usage<T>
where
    T::CapacityValue: Clone,
{
    fn clone(&self) -> Self {
        Self {
            resource_id: self.resource_id,
            capacity: self.capacity.clone(),
        }
    }
}

impl<T: Resource> std::fmt::Debug for Usage<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Usage")
            .field("resource_id", &self.resource_id)
            .finish_non_exhaustive()
    }
}

/// Ergonomic conversion into `Usage<T>`.
///
/// Allows passing resource references and capacity values directly instead
/// of constructing `Usage<T>` structs manually:
/// - Unit resources: pass any `R: Into<Ref<T>>` (e.g., a handle reference)
/// - Capacity resources: pass `(R, cap_value1, cap_value2, ...)`
pub trait IntoUsage<T: Resource> {
    fn into_usage(self) -> Usage<T>;
}

// Unit resource: any R that converts to Ref<T>, when CapacityValue is Default.
impl<T: Resource, R: Into<Ref<T>>> IntoUsage<T> for R
where
    T::CapacityValue: Default,
{
    fn into_usage(self) -> Usage<T> {
        Usage {
            resource_id: self.into(),
            capacity: Default::default(),
        }
    }
}

// Single capacity field: (ref, value).
impl<T: Resource, R: Into<Ref<T>>, V1> IntoUsage<T> for (R, V1)
where
    T::CapacityValue: From<(V1,)>,
{
    fn into_usage(self) -> Usage<T> {
        Usage {
            resource_id: self.0.into(),
            capacity: (self.1,).into(),
        }
    }
}

// Two capacity fields: (ref, value1, value2).
impl<T: Resource, R: Into<Ref<T>>, V1, V2> IntoUsage<T> for (R, V1, V2)
where
    T::CapacityValue: From<(V1, V2)>,
{
    fn into_usage(self) -> Usage<T> {
        Usage {
            resource_id: self.0.into(),
            capacity: (self.1, self.2).into(),
        }
    }
}

/// Convenience function to convert an `IntoUsage<T>` value into `Usage<T>`.
///
/// Use inside `Some(...)` when calling state transition methods:
/// ```ignore
/// use quent_model::usage;
/// task.computing(Some(usage(&thread)), None);
/// task.computing(Some(usage((&mem_pool, 1024))), None);
/// ```
pub fn usage<T: Resource>(value: impl IntoUsage<T>) -> Usage<T> {
    value.into_usage()
}
