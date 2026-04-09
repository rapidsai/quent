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
