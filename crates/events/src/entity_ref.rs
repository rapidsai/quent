// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Reference from one entity instance to another.

use std::marker::PhantomData;

use uuid::Uuid;

/// Reference to an entity instance of type `E`, optionally carrying payload
/// data `T`.
///
/// In instrumentation libraries generated with `instrumentation-build`, `E` is
/// typically a marker type.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct EntityRef<E, T = ()> {
    #[cfg_attr(feature = "serde", serde(skip))]
    _entity: PhantomData<E>,
    /// Identifier of the referenced entity instance.
    pub target: Uuid,
    /// Payload carried alongside the reference.
    pub data: T,
}

impl<E, T> EntityRef<E, T> {
    /// A reference to the entity instance identified by `target`, carrying `data`.
    pub fn new(target: Uuid, data: T) -> Self {
        Self {
            _entity: PhantomData,
            target,
            data,
        }
    }
}

/// Entity marker for a reference not restricted to a single entity type.
///
/// Untargeted entity reference fields in instrumentation have the type:
/// `EntityRef<AnyEntity, ...>`.
#[derive(Debug, Clone, Copy)]
pub struct AnyEntity;
