// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed entity identities.

use std::marker::PhantomData;

use crate::{EntityMarker, EntityRef, Uuid};

/// An entity UUID scoped to its entity marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id<E: EntityMarker> {
    uuid: Uuid,
    marker: PhantomData<fn() -> E>,
}

impl<E: EntityMarker> Id<E> {
    /// Creates a typed identity without checking whether the entity exists.
    pub fn new(uuid: Uuid) -> Self {
        Self {
            uuid,
            marker: PhantomData,
        }
    }

    /// Returns the entity UUID.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

impl<E: EntityMarker> From<Uuid> for Id<E> {
    fn from(uuid: Uuid) -> Self {
        Self::new(uuid)
    }
}

impl<E: EntityMarker, T> From<EntityRef<E, T>> for Id<E> {
    fn from(reference: EntityRef<E, T>) -> Self {
        Self::new(reference.target)
    }
}

impl<E: EntityMarker, T> From<&EntityRef<E, T>> for Id<E> {
    fn from(reference: &EntityRef<E, T>) -> Self {
        Self::new(reference.target)
    }
}
