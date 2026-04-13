// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Capacity type for resource definitions.
//!
//! `Capacity<V, K>` wraps a value `V` with a kind marker `K` that indicates
//! whether the capacity represents occupancy (amount held during a span) or
//! rate (total quantity processed over a span).
//!
//! `V` is restricted to `u64` or `Option<u64>` (the spec requires
//! non-negative integer bounds).

/// Marker for occupancy-type capacity: usage value represents the amount held during a Span.
#[derive(Debug, Clone, Copy)]
pub struct Occupancy;

/// Marker for rate-type capacity: usage value represents total quantity processed over a Span.
#[derive(Debug, Clone, Copy)]
pub struct Rate;

/// Sealed trait restricting capacity value types to `u64` and `Option<u64>`.
pub trait CapacityBound: private::Sealed {}

impl CapacityBound for u64 {}
impl CapacityBound for Option<u64> {}

mod private {
    pub trait Sealed {}
    impl Sealed for u64 {}
    impl Sealed for Option<u64> {}
}

/// A capacity value on a resource. `V` is the value type (`u64` or
/// `Option<u64>`), `K` is the kind marker (`Occupancy` or `Rate`).
///
/// The kind marker is erased at runtime (zero-cost phantom type).
/// Serialization is transparent: `Capacity<u64>` serializes as a plain `u64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Capacity<V: CapacityBound, K = Occupancy> {
    pub value: V,
    #[cfg_attr(feature = "serde", serde(skip))]
    _kind: std::marker::PhantomData<K>,
}

impl<V: CapacityBound, K> Capacity<V, K> {
    pub fn new(value: V) -> Self {
        Self {
            value,
            _kind: std::marker::PhantomData,
        }
    }
}
