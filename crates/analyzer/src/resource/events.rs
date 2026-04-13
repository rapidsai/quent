// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generic analyzed Resource reconstructed from model-generated resource events.
//!
//! `ResourceEvents<T>` wraps an `FsmEvents<T>` and adds resource-specific
//! data extracted from the Initializing transition: `parent_group_id` and
//! `resource_type_name`.

use quent_events::Event;
use quent_model::{FsmEvent, analyze::TransitionInfo};
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{
        Fsm, FsmTypeDecl, FsmTypeDeclaration, FsmUsages,
        events::{FsmEvents, FsmEventsBuilder, TransitionEvent},
    },
    resource::{Resource, Usage, Using},
};

/// A generic analyzed resource reconstructed from model-generated resource events.
///
/// `T` is the transition enum (e.g., `MemoryTransition`), which implements
/// `TransitionInfo`. Wraps `FsmEvents<T>` and adds:
/// - `parent_group_id`: extracted from the Initializing state data
/// - `resource_type_name`: extracted from the Initializing state data
///
/// The `Entity::type_name()` returns `resource_type_name` (the user-specified
/// resource type, e.g. "filesystem") rather than the FSM type name (e.g. "memory").
pub struct ResourceEvents<T: TransitionInfo> {
    inner: FsmEvents<T>,
    parent_group_id: Uuid,
    resource_type_name: String,
}

impl<T: TransitionInfo> std::fmt::Debug for ResourceEvents<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceEvents")
            .field("inner", &self.inner)
            .field("parent_group_id", &self.parent_group_id)
            .field("resource_type_name", &self.resource_type_name)
            .finish()
    }
}

impl<T: TransitionInfo> ResourceEvents<T> {
    /// Access the inner FSM.
    pub fn inner(&self) -> &FsmEvents<T> {
        &self.inner
    }
}

impl<T: TransitionInfo> Entity for ResourceEvents<T> {
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

impl<T: TransitionInfo> Resource for ResourceEvents<T> {
    fn parent_group_id(&self) -> Uuid {
        self.parent_group_id
    }
}

impl<T: TransitionInfo> Fsm for ResourceEvents<T> {
    type TransitionType = TransitionEvent<T>;
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.inner.transition(index)
    }
}

impl<'a, T: TransitionInfo + 'a> FsmUsages<'a> for ResourceEvents<T> {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
        self.inner.usages_with_state_names()
    }
}

impl<T: TransitionInfo> Using for ResourceEvents<T> {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        self.inner.usages()
    }
}

impl<T: TransitionInfo> FsmTypeDeclaration for ResourceEvents<T> {
    fn fsm_type_declaration() -> FsmTypeDecl {
        FsmEvents::<T>::fsm_type_declaration()
    }
}

/// Builder for `ResourceEvents<T>`.
///
/// Wraps `FsmEventsBuilder<T>` and extracts `parent_group_id` and
/// `resource_type_name` from the first (Initializing) transition.
pub struct ResourceEventsBuilder<T: TransitionInfo> {
    inner: FsmEventsBuilder<T>,
    parent_group_id: Option<Uuid>,
    resource_type_name: Option<String>,
}

impl<T: TransitionInfo> ResourceEventsBuilder<T> {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self {
            inner: FsmEventsBuilder::try_new(id)?,
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

    pub fn push(&mut self, event: Event<FsmEvent<T>>) {
        self.inner.push(event);
    }

    pub fn try_build(self) -> AnalyzerResult<ResourceEvents<T>> {
        let id = self.inner.id();
        let inner = self.inner.try_build()?;
        let parent_group_id = self.parent_group_id.ok_or_else(|| {
            AnalyzerError::IncompleteEntity(format!("resource {} must have a parent group id", id))
        })?;
        let resource_type_name = self.resource_type_name.ok_or_else(|| {
            AnalyzerError::IncompleteEntity(format!(
                "resource {} must have a resource type name",
                id
            ))
        })?;
        Ok(ResourceEvents {
            inner,
            parent_group_id,
            resource_type_name,
        })
    }
}
