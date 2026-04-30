// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! FSM-related functionality

use quent_attributes::Attribute;
use quent_time::{Timestamp, span::SpanUnixNanoSec};
#[cfg(feature = "ts")]
use serde::Serialize;
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{AnalyzerResult, Entity, Span, error::AnalyzerError, resource::Usage};

pub mod collection;
pub mod events;
pub mod runtime;

/// Trait for types that represent an [`Fsm`] `State` transition.
pub trait Transition: Timestamp {
    /// Return the unique name of the state this transition leads to.
    fn name(&self) -> &str;
    /// Return an iterator over arbitrary key-value attributes associated with
    /// this transition.
    fn attributes(&self) -> impl Iterator<Item = &Attribute>;
}

/// Trait for types that represent a Finite State Machine (FSM).
///
/// An FSM is modeled as a sequence of transitions between uniquely named
/// states. Each FSM must have at least two transition, some entry transition
/// and an exit transition. The number of states is always one less than the
/// number of transitions.
pub trait Fsm: Entity {
    /// The type of transitions stored by this FSM.
    ///
    /// This associated type enables dyn-free access to underlying transition
    /// data.
    type TransitionType: Transition;

    /// Return the number of states in this FSM.
    ///
    /// This is always the number of transitions - 1, since the final transition
    /// must be into the special exit state.
    fn len(&self) -> usize;

    /// Return true if this FSM has no states (meaning the model of whatever it
    /// represents is incomplete).
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return a reference to the transition at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    fn transition(&self, index: usize) -> Option<&Self::TransitionType>;

    /// Return a reference to the state at the given index.
    ///
    /// The state spans from transition `index` to transition `index + 1`.
    /// Returns `None` if the index is out of bounds.
    fn state<'a>(&'a self, index: usize) -> Option<FsmStateRef<'a, Self, Self::TransitionType>> {
        (self.len() > index).then_some(FsmStateRef { fsm: self, index })
    }

    /// Return an iterator over all states in this FSM.
    fn states<'a>(
        &'a self,
    ) -> impl ExactSizeIterator<Item = FsmStateRef<'a, Self, Self::TransitionType>> {
        (0..self.len()).map(|index| self.state(index).unwrap())
    }

    /// Return the first state, if the FSM is not empty.
    fn first<'a>(&'a self) -> Option<FsmStateRef<'a, Self, Self::TransitionType>> {
        self.state(0)
    }

    /// Return the last state, if the FSM is not empty.
    fn last<'a>(&'a self) -> Option<FsmStateRef<'a, Self, Self::TransitionType>> {
        self.state(self.len() - 1)
    }
}

/// Trait for FSMs that have resource usages associated with their states.
pub trait FsmUsages<'a>: Fsm {
    /// Return an iterator over all usages with their associated state names.
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)>;
}

impl<U> Span for U
where
    U: Fsm,
{
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        if let Some(start) = self.first().map(|s| s.span().start())
            && let Some(end) = self.last().map(|s| s.span().end())
        {
            Ok(SpanUnixNanoSec::try_new(start, end)?)
        } else {
            Err(AnalyzerError::IncompleteEntity(format!(
                "fsm '{}' (id={}) is incomplete",
                self.type_name(),
                self.id()
            )))
        }
    }
}

#[derive(Clone)]
pub struct FsmStateRef<'a, F, T>
where
    F: Fsm<TransitionType = T> + ?Sized,
    T: Transition,
{
    fsm: &'a F,
    index: usize,
}

impl<'a, F, T> FsmStateRef<'a, F, T>
where
    F: Fsm<TransitionType = T>,
    T: Transition,
{
    pub fn name(&self) -> &str {
        self.fsm.transition(self.index).unwrap().name()
    }

    pub fn span(&self) -> SpanUnixNanoSec {
        let start = self.fsm.transition(self.index).unwrap().timestamp();
        let end = self.fsm.transition(self.index + 1).unwrap().timestamp();
        SpanUnixNanoSec::try_new(start, end).unwrap()
    }

    pub fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.fsm.transition(self.index).unwrap().attributes()
    }
}

/// Trait for FSM types to deliver a run-time definition of their states and possible transitions.
pub trait FsmTypeDeclaration {
    fn fsm_type_declaration() -> FsmTypeDecl;
}

/// A declaration of an FSM state.
#[derive(Debug)]
#[cfg_attr(feature = "ts", derive(Serialize, TS))]
pub struct FsmStateTypeDecl {
    /// The name of this FSM state.
    pub name: String,
    // TODO(johanpel): figure out how to best do this
    // The attributes this FSM state can have.
    // pub attributes: Vec<Attribute>,
    /// The names of the resource types this FSM state can use.
    pub usages: Vec<String>,
}

/// A declaration of an FSM state transition.
#[derive(Debug)]
#[cfg_attr(feature = "ts", derive(Serialize, TS))]
pub enum FsmTransitionDecl {
    /// Initial transition into the state with this name.
    Entry(String),
    /// Transition from a state to a state with these names (from, to).
    Transition(String, String),
    /// Exit transition from the state with this name.
    Exit(String),
}

/// A declaration of an FSM type.
#[derive(Debug)]
#[cfg_attr(feature = "ts", derive(Serialize, TS))]
pub struct FsmTypeDecl {
    /// The name of this FSM type.
    pub name: String,
    /// The states of this FSM type.
    pub states: Vec<FsmStateTypeDecl>,
    /// The possible transitions of this FSM type.
    pub transitions: Vec<FsmTransitionDecl>,
}
