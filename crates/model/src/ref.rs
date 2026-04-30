// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed entity references.
//!
//! `Ref<T>` is a newtype over `Uuid` that provides compile-time type safety
//! for entity references. It resolves to `Uuid` on the wire.

use std::fmt;
use std::marker::PhantomData;

use uuid::Uuid;

/// A typed reference to an entity, FSM, or resource instance.
///
/// `Ref<T>` wraps a `Uuid` and carries a phantom type parameter `T` that
/// identifies the referenced type. This prevents accidentally passing the
/// wrong entity's ID where a specific type is expected.
///
/// The type parameter is erased at serialization time — on the wire, a
/// `Ref<T>` is just a `Uuid`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Ref<T> {
    id: Uuid,
    #[cfg_attr(feature = "serde", serde(skip))]
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Ref<T> {
    /// Creates a new typed reference from a `Uuid`.
    pub const fn new(id: Uuid) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }

    /// Returns the underlying `Uuid`.
    pub const fn uuid(&self) -> Uuid {
        self.id
    }
}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Ref<T> {}

impl<T> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Ref<T> {}

impl<T> std::hash::Hash for Ref<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> fmt::Debug for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ref<{}>({})", std::any::type_name::<T>(), self.id)
    }
}

impl<T> fmt::Display for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl<T> From<Ref<T>> for Uuid {
    fn from(r: Ref<T>) -> Uuid {
        r.id
    }
}
