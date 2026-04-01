// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generic analyzed Resource reconstructed from model-generated resource events.
//!
//! `AnalyzedResource<T>` wraps an `AnalyzedFsm<T>` and adds resource-specific
//! data extracted from the Initializing transition: `parent_group_id` and
//! `resource_type_name`.

use quent_events::Event;
use quent_model::{FsmEvent, analyze::TransitionInfo};
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{
        Fsm, FsmTypeDecl, FsmTypeDeclaration, FsmUsages,
        analyzed::{AnalyzedFsm, AnalyzedFsmBuilder, AnalyzedTransition},
    },
    resource::{Resource, Usage, Using},
};

/// A generic analyzed resource reconstructed from model-generated resource events.
///
/// `T` is the transition enum (e.g., `MemoryTransition`), which implements
/// `TransitionInfo`. Wraps `AnalyzedFsm<T>` and adds:
/// - `parent_group_id`: extracted from the Initializing state data
/// - `resource_type_name`: extracted from the Initializing state data
///
/// The `Entity::type_name()` returns `resource_type_name` (the user-specified
/// resource type, e.g. "filesystem") rather than the FSM type name (e.g. "memory").
#[derive(Debug)]
pub struct AnalyzedResource<T: TransitionInfo + std::fmt::Debug> {
    inner: AnalyzedFsm<T>,
    parent_group_id: Uuid,
    resource_type_name: String,
}

impl<T: TransitionInfo + std::fmt::Debug> AnalyzedResource<T> {
    /// Access the inner FSM.
    pub fn inner(&self) -> &AnalyzedFsm<T> {
        &self.inner
    }
}

// --- Entity ---

impl<T: TransitionInfo + std::fmt::Debug> Entity for AnalyzedResource<T> {
    fn id(&self) -> Uuid {
        self.inner.id()
    }
    fn type_name(&self) -> &str {
        &self.resource_type_name
    }
    fn instance_name(&self) -> &str {
        self.inner.instance_name()
    }
}

// --- Resource ---

impl<T: TransitionInfo + std::fmt::Debug> Resource for AnalyzedResource<T> {
    fn parent_group_id(&self) -> Uuid {
        self.parent_group_id
    }
}

// --- Fsm (delegate to inner) ---

impl<T: TransitionInfo + std::fmt::Debug> Fsm for AnalyzedResource<T> {
    type TransitionType = AnalyzedTransition<T>;
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.inner.transition(index)
    }
}

// --- FsmUsages (delegate to inner) ---

impl<'a, T: TransitionInfo + std::fmt::Debug + 'a> FsmUsages<'a> for AnalyzedResource<T> {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
        self.inner.usages_with_state_names()
    }
}

// --- Using (delegate to inner) ---

impl<T: TransitionInfo + std::fmt::Debug> Using for AnalyzedResource<T> {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        self.inner.usages()
    }
}

// --- FsmTypeDeclaration (delegate to inner) ---

impl<T: TransitionInfo + std::fmt::Debug> FsmTypeDeclaration for AnalyzedResource<T> {
    fn fsm_type_declaration() -> FsmTypeDecl {
        AnalyzedFsm::<T>::fsm_type_declaration()
    }
}

/// Builder for `AnalyzedResource<T>`.
///
/// Wraps `AnalyzedFsmBuilder<T, D>` and extracts `parent_group_id` and
/// `resource_type_name` from the first (Initializing) transition.
pub struct AnalyzedResourceBuilder<T: TransitionInfo, D> {
    inner: AnalyzedFsmBuilder<T, D>,
    parent_group_id: Option<Uuid>,
    resource_type_name: Option<String>,
}

impl<T: TransitionInfo, D> AnalyzedResourceBuilder<T, D> {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self {
            inner: AnalyzedFsmBuilder::try_new(id)?,
            parent_group_id: None,
            resource_type_name: None,
        })
    }

    /// Set the parent group ID (typically extracted from the Initializing event).
    pub fn set_parent_group_id(&mut self, id: Uuid) {
        self.parent_group_id = Some(id);
    }

    /// Set the resource type name (typically extracted from the Initializing event).
    pub fn set_resource_type_name(&mut self, name: String) {
        self.resource_type_name = Some(name);
    }

    pub fn push(&mut self, event: Event<FsmEvent<T, D>>) {
        self.inner.push(event);
    }

    pub fn try_build(self) -> AnalyzerResult<AnalyzedResource<T>>
    where
        T: std::fmt::Debug,
    {
        let id = self.inner.id();
        let inner = self.inner.try_build()?;
        let parent_group_id = self.parent_group_id.ok_or_else(|| {
            AnalyzerError::IncompleteEntity(format!(
                "resource {} must have a parent group id",
                id
            ))
        })?;
        let resource_type_name = self.resource_type_name.ok_or_else(|| {
            AnalyzerError::IncompleteEntity(format!(
                "resource {} must have a resource type name",
                id
            ))
        })?;
        Ok(AnalyzedResource {
            inner,
            parent_group_id,
            resource_type_name,
        })
    }
}
