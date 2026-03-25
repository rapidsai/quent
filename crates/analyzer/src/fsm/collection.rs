// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Collections of FSMs

use std::collections::HashSet;

use rustc_hash::FxHashMap as HashMap;
use uuid::Uuid;

use crate::{
    fsm::{Fsm, Transition},
    resource::{Usage, Using},
};

/// Trait for types that hold a collection of [`Fsm`]s.
pub trait FsmCollection<F, T>
where
    F: Fsm<TransitionType = T>,
    T: Transition,
{
    fn fsms<'a>(&'a self) -> impl Iterator<Item = &'a F> + 'a
    where
        F: 'a;

    fn contains_fsm_type(&self, type_name: &str) -> bool;
}

/// An in-memory collection of [`Fsm`]s.
pub struct InMemoryFsms<F, T>
where
    F: Fsm<TransitionType = T>,
    T: Transition,
{
    pub fsms: HashMap<Uuid, F>,
    pub fsm_type_names: HashSet<String>,
}

impl<F, T> FsmCollection<F, T> for InMemoryFsms<F, T>
where
    F: Fsm<TransitionType = T>,
    T: Transition,
{
    fn fsms<'a>(&'a self) -> impl Iterator<Item = &'a F> + 'a
    where
        T: 'a,
    {
        self.fsms.values()
    }

    fn contains_fsm_type(&self, type_name: &str) -> bool {
        self.fsm_type_names.contains(type_name)
    }
}

impl<F, T> Using for InMemoryFsms<F, T>
where
    F: Fsm<TransitionType = T> + Using,
    T: Transition,
{
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        self.fsms.values().flat_map(|fsm| fsm.usages())
    }
}

#[cfg(test)]
impl<F, T> InMemoryFsms<F, T>
where
    F: Fsm<TransitionType = T>,
    T: Transition,
{
    pub(crate) fn new() -> Self {
        Self {
            fsms: Default::default(),
            fsm_type_names: Default::default(),
        }
    }
    pub(crate) fn insert(&mut self, fsm: F) {
        self.fsm_type_names.insert(fsm.type_name().to_owned());
        self.fsms.insert(fsm.id(), fsm);
    }
}
