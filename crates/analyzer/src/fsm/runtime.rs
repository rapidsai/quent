// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Run-time defined FSMs (in analysis)

use std::collections::HashSet;

use rustc_hash::FxHashMap as HashMap;

use quent_attributes::Attribute;
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{
        Fsm, FsmStateRef, FsmStateTypeDecl, FsmTransitionDecl, FsmTypeDecl, FsmUsages, Transition,
        collection::InMemoryFsms,
    },
    resource::{CapacityValue, Usage, Using, collection::ResourceCollection},
};

/// A run-time defined [`Usage`] of a `Resource` in an [`Fsm`] `State`.
#[derive(Clone, Debug, PartialEq)]
pub struct RtFsmStateUsage {
    pub resource: Uuid,
    pub capacities: SmallVec<[CapacityValue; 1]>,
}

impl RtFsmStateUsage {
    pub fn new(resource: Uuid, capacities: impl Into<SmallVec<[CapacityValue; 1]>>) -> Self {
        Self {
            resource,
            capacities: capacities.into(),
        }
    }

    pub fn unit(resource: Uuid) -> Self {
        Self {
            resource,
            capacities: SmallVec::from([CapacityValue::new("unit", 1)]),
        }
    }
}

/// A run-time defined `StateTransition` of an [`Fsm`].
pub struct RtFsmTransition {
    pub name: String,
    pub usages: Vec<RtFsmStateUsage>,
    pub timestamp: TimeUnixNanoSec,
    pub attributes: Vec<Attribute>,
}

impl Timestamp for RtFsmTransition {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl Transition for RtFsmTransition {
    fn name(&self) -> &str {
        self.name.as_str()
    }
    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes.iter()
    }
}

/// Builder for run-time defined [`Fsm`]s with `State`s of type T.
pub struct RtFsmBuilder<T> {
    id: Uuid,
    type_name: Option<String>,
    instance_name: Option<String>,
    transitions: TimeOrderedCollector<T>,
}

impl<T> RtFsmBuilder<T> {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            type_name: None,
            instance_name: None,
            transitions: TimeOrderedCollector::default(),
        }
    }
    pub fn set_type_name(&mut self, type_name: String) -> &mut Self {
        self.type_name = Some(type_name);
        self
    }
    pub fn set_instance_name(&mut self, instance_name: String) -> &mut Self {
        self.instance_name = Some(instance_name);
        self
    }
}

impl<T> RtFsmBuilder<T>
where
    T: Timestamp,
{
    pub fn push(&mut self, state: T) {
        self.transitions.push(state)
    }
}

impl<T> Extend<T> for RtFsmBuilder<T>
where
    T: Timestamp,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl RtFsmBuilder<RtFsmTransition> {
    pub fn try_build(self) -> AnalyzerResult<RtFsm> {
        // Ensure we have >= 2 transitions
        let transitions = self.transitions.into_inner();
        // Ensure exit state per spec
        let last_name = &transitions.last().unwrap(/*len checked above*/).name;
        if last_name != "exit" {
            Err(AnalyzerError::Validation(format!(
                "final state of fsm {} is named {}, but must be named \"exit\"",
                self.id, last_name,
            )))
        } else {
            // For runtime-defined FSMs, there is no compile-time defined
            // transition logic to check.
            Ok(RtFsm {
                id: self.id,
                type_name: self.type_name.ok_or_else(|| {
                    AnalyzerError::IncompleteEntity(format!("fsm {} has no type name", self.id))
                })?,
                instance_name: self.instance_name.ok_or_else(|| {
                    AnalyzerError::IncompleteEntity(format!("fsm {} has no instance name", self.id))
                })?,
                transitions,
            })
        }
    }
}

/// Run-time defined Finite-State-Machine
pub struct RtFsm {
    id: Uuid,
    type_name: String,
    instance_name: String,
    transitions: Vec<RtFsmTransition>,
}

impl Fsm for RtFsm {
    type TransitionType = RtFsmTransition;

    fn len(&self) -> usize {
        self.transitions.len() - 1 // -1 for the exit transition.
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.transitions.get(index)
    }
}

impl<'a> FsmUsages<'a> for RtFsm {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
        self.transitions.windows(2).flat_map(|window| {
            let name = window[0].name.as_str();
            let start = window[0].timestamp();
            let end = window[1].timestamp();
            let span = SpanUnixNanoSec::try_new(start, end).unwrap();
            window[0].usages.iter().map(move |u| (name, (u, span)))
        })
    }
}

impl RtFsm {
    pub fn transitions(&self) -> &[RtFsmTransition] {
        &self.transitions
    }

    pub fn try_declaration(
        &self,
        resources: &impl ResourceCollection,
    ) -> AnalyzerResult<FsmTypeDecl> {
        let mut state_decls: HashMap<&str, FsmStateTypeDecl> = HashMap::default();

        for transition in &self.transitions {
            let entry = state_decls
                .entry(transition.name.as_str())
                .or_insert_with(|| FsmStateTypeDecl {
                    name: transition.name.clone(),
                    usages: Vec::new(),
                });
            for usage in &transition.usages {
                let resource = resources.resource(usage.resource)?;
                let resource_type_name = &resources.resource_type(resource.type_name())?.name;
                if !entry.usages.contains(resource_type_name) {
                    entry.usages.push(resource_type_name.clone());
                }
            }
        }

        let mut transitions = Vec::new();
        if let Some(first_transition) = self.transitions.first() {
            transitions.push(FsmTransitionDecl::Entry(first_transition.name.clone()));
        }
        for window in self.transitions.windows(2) {
            transitions.push(FsmTransitionDecl::Transition(
                window[0].name.clone(),
                window[1].name.clone(),
            ));
        }
        if let Some(last_transition) = self.transitions.last() {
            transitions.push(FsmTransitionDecl::Exit(last_transition.name.clone()));
        }

        Ok(FsmTypeDecl {
            name: self.type_name.clone(),
            states: state_decls.into_values().collect(),
            transitions,
        })
    }
}

#[cfg(test)]
impl RtFsm {
    pub fn try_new(
        id: Uuid,
        type_name: impl Into<String>,
        instance_name: impl Into<String>,
        transitions: impl IntoIterator<Item = RtFsmTransition>,
    ) -> AnalyzerResult<RtFsm> {
        let mut builder = RtFsmBuilder::new(id);
        builder.set_type_name(type_name.into());
        builder.set_instance_name(instance_name.into());
        builder.extend(transitions);
        builder.try_build()
    }
}

impl Entity for RtFsm {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        self.type_name.as_str()
    }

    fn instance_name(&self) -> &str {
        self.instance_name.as_str()
    }
}

impl<'a> Usage<'a> for (&'a RtFsmStateUsage, SpanUnixNanoSec) {
    fn entity_id(&self) -> Uuid {
        self.0.resource
    }
    fn resource_id(&self) -> Uuid {
        self.0.resource
    }
    fn capacities(&self) -> impl Iterator<Item = &'a CapacityValue> {
        self.0.capacities.iter()
    }
    fn span(&self) -> SpanUnixNanoSec {
        self.1
    }
}

impl<'a> Using for FsmStateRef<'a, RtFsm, RtFsmTransition> {
    fn usages<'b>(&'b self) -> impl Iterator<Item = impl Usage<'b>> {
        let span = self.span();
        self.fsm.transitions[self.index]
            .usages
            .iter()
            .map(move |usage| (usage, span))
    }
}

impl Using for RtFsm {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        self.transitions.windows(2).flat_map(|window| {
            let start = window[0].timestamp();
            let end = window[1].timestamp();
            let span = SpanUnixNanoSec::try_new(start, end).unwrap();
            window[0].usages.iter().map(move |u| (u, span))
        })
    }
}

#[derive(Default)]
pub struct RtFsmsBuilder {
    fsms: HashMap<Uuid, RtFsmBuilder<RtFsmTransition>>,
}

impl RtFsmsBuilder {
    pub fn push(&mut self, id: Uuid, transition: RtFsmTransition) {
        self.fsms
            .entry(id)
            .or_insert_with(|| RtFsmBuilder::new(id))
            .push(transition);
    }

    pub fn try_build(self) -> AnalyzerResult<InMemoryFsms<RtFsm, RtFsmTransition>> {
        // Build all FSMs.
        let mut fsms: HashMap<Uuid, RtFsm> =
            HashMap::with_capacity_and_hasher(self.fsms.capacity(), Default::default());
        let mut fsm_type_names = HashSet::<String>::new();

        for (k, fsm) in self.fsms.into_iter() {
            // TODO(johanpel): for now bubble up this error but if there are
            // e.g. abrupt failures we may want to move incomplete FSMs into
            // their own bucket.
            let fsm = fsm.try_build()?;
            if !fsm_type_names.contains(fsm.type_name()) {
                fsm_type_names.insert(fsm.type_name().to_owned());
            }
            fsms.insert(k, fsm);
        }

        Ok(InMemoryFsms {
            fsms,
            fsm_type_names,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_span() {
        // Create an FSM with 3 state transitions plus an exit state
        let fsm = RtFsm::try_new(
            Uuid::now_v7(),
            "test",
            "test",
            [
                RtFsmTransition {
                    name: "a".to_string(),
                    usages: vec![],
                    timestamp: 1,
                    attributes: vec![],
                },
                RtFsmTransition {
                    name: "b".to_string(),
                    usages: vec![],
                    timestamp: 2,
                    attributes: vec![],
                },
                RtFsmTransition {
                    name: "c".to_string(),
                    usages: vec![],
                    timestamp: 3,
                    attributes: vec![],
                },
                RtFsmTransition {
                    name: "exit".to_string(),
                    usages: vec![],
                    timestamp: 4,
                    attributes: vec![],
                },
            ],
        )
        .unwrap();

        let state = fsm.state(0).unwrap();
        assert_eq!(state.name(), "a");
        assert_eq!(state.span().start(), 1);
        assert_eq!(state.span().end(), 2);

        let span = fsm.state(1).unwrap();
        assert_eq!(span.name(), "b");
        assert_eq!(span.span().start(), 2);
        assert_eq!(span.span().end(), 3);

        let span = fsm.state(2).unwrap();
        assert_eq!(span.name(), "c");
        assert_eq!(span.span().start(), 3);
        assert_eq!(span.span().end(), 4);

        assert!(fsm.state(3).is_none());
        assert!(fsm.state(usize::MAX).is_none());

        assert_eq!(fsm.states().len(), 3);
        for (index, state_span) in fsm.states().enumerate() {
            assert_eq!(state_span.span().start(), 1 + index as u64);
            assert_eq!(state_span.span().end(), 2 + index as u64);
            assert_eq!(state_span.name(), ["a", "b", "c"][index]);
        }
    }

    #[test]
    fn incomplete_fsm() {
        // Create an FSM with 1 state transition. This is not complete.
        assert!(
            RtFsm::try_new(
                Uuid::now_v7(),
                "test",
                "test",
                [RtFsmTransition {
                    name: "a".to_string(),
                    usages: vec![],
                    timestamp: 1,
                    attributes: vec![]
                }],
            )
            .is_err()
        );
    }
}
