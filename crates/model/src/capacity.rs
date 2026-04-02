// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Capacity type for resource definitions.
//!
//! `Capacity<V, K>` wraps a value `V` with a kind marker `K` that indicates
//! whether the capacity represents occupancy (amount held during a span) or
//! rate (total quantity processed over a span).

/// Marker for occupancy-type capacity: usage value represents the amount held during a Span.
#[derive(Debug, Clone, Copy)]
pub struct Occupancy;

/// Marker for rate-type capacity: usage value represents total quantity processed over a Span.
#[derive(Debug, Clone, Copy)]
pub struct Rate;

/// A capacity value on a resource. `V` is the value type, `K` is the kind marker.
///
/// The kind marker (`Occupancy` or `Rate`) is erased at runtime (zero-cost
/// phantom type). Serialization is transparent: `Capacity<u64>` serializes
/// as a plain `u64`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Capacity<V, K = Occupancy> {
    pub value: V,
    #[serde(skip)]
    _kind: std::marker::PhantomData<K>,
}

impl<V, K> Capacity<V, K> {
    pub fn new(value: V) -> Self {
        Self {
            value,
            _kind: std::marker::PhantomData,
        }
    }
}

impl<V, K> std::ops::Deref for Capacity<V, K> {
    type Target = V;
    fn deref(&self) -> &V {
        &self.value
    }
}
