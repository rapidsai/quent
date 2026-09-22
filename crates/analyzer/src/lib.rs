// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Telemetry analysis functionality based on modeling primitives.

pub use crate::error::AnalyzerError;
pub use entity::Entity;
use quent_time::span::SpanUnixNanoSec;
pub use ref_tree::RefTreeEntity;
use uuid::Uuid;

pub mod context;
pub mod entity;
pub mod error;
pub mod fsm;
pub mod ref_tree;
pub mod resource;
#[cfg(feature = "service")]
pub mod service;
pub mod timeline;

pub type AnalyzerResult<T> = std::result::Result<T, AnalyzerError>;

/// Trait for things that are associated with a span of time.
///
/// Typically represents the entire lifetime of the entity.
pub trait Span {
    /// Return the span of time this type is associated with.
    ///
    /// # Errors
    ///
    /// This function can return an [`AnalyzerError`] in cases such as:
    /// - Events are missing to form a complete entity model.
    /// - The sequence of FSM transition events violates model specifications.
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec>;
}

/// Trait for application models.
pub trait Model {
    /// Type-safety wrapper around an entity ID.
    type EntityIdType;

    /// Given an [`Entity`] ID, resolve it into an [`Self::EntityIdType`].
    fn try_entity_ref(&self, entity_id: Uuid) -> AnalyzerResult<Self::EntityIdType>;
}
