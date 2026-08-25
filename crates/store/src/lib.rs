// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Typed access to stored data.

pub mod entity;
pub mod event;

pub use entity::{
    AnyEntityHandle, ContextSet, ContextSetError, EntityHandle, EntityStore, ModelEntityStore,
};
