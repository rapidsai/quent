//! Collections of FSMs

use std::collections::HashSet;

use rustc_hash::FxHashMap as HashMap;

use quent_time::span::SpanUnixNanoSec;
use uuid::Uuid;

use crate::{
    fsm::Fsm,
    resource::{Usage, Using},
};

/// Trait for types that hold a collection of [`Fsm`]s.
pub trait FsmCollection<T>
where
    T: Fsm,
{
    fn fsms<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a
    where
        T: 'a;

    fn contains_fsm_type(&self, type_name: &str) -> bool;
}

/// An in-memory collection of [`Fsm`]s.
pub struct InMemoryFsms<T>
where
    T: Fsm,
{
    pub fsms: HashMap<Uuid, T>,
    pub fsm_type_names: HashSet<String>,
}

impl<T> FsmCollection<T> for InMemoryFsms<T>
where
    T: Fsm,
{
    fn fsms<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a
    where
        T: 'a,
    {
        self.fsms.values()
    }

    fn contains_fsm_type(&self, type_name: &str) -> bool {
        self.fsm_type_names.contains(type_name)
    }
}

impl<T> Using for InMemoryFsms<T>
where
    T: Fsm + Using,
{
    fn usages(&self) -> impl Iterator<Item = (&Usage, SpanUnixNanoSec)> {
        self.fsms.values().flat_map(|fsm| fsm.usages())
    }
}

#[cfg(test)]
impl<T> InMemoryFsms<T>
where
    T: Fsm,
{
    pub(crate) fn new() -> Self {
        Self {
            fsms: Default::default(),
            fsm_type_names: Default::default(),
        }
    }
    pub(crate) fn insert(&mut self, fsm: T) {
        self.fsm_type_names.insert(fsm.type_name().to_owned());
        self.fsms.insert(fsm.id(), fsm);
    }
}
