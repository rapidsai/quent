//! Run-time defined FSMs (in analysis)

use std::collections::HashSet;

use rustc_hash::FxHashMap as HashMap;

use quent_attributes::Attribute;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, OrderedStateTransitionCollector, State, Transition, collection::InMemoryFsms},
    resource::{Usage, Using},
};

/// A run-time defined [`StateTransition`] of an [`Fsm`].
pub struct RtTransition {
    pub name: String,
    pub usages: Vec<Usage>,
    pub timestamp: TimeUnixNanoSec,
    pub attributes: Vec<Attribute>,
}

impl Transition for RtTransition {
    type Target = RtState;

    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }

    fn try_into_state(self, end: TimeUnixNanoSec) -> AnalyzerResult<Self::Target> {
        Ok(RtState {
            name: self.name,
            usages: self.usages,
            span: SpanUnixNanoSec::try_new(self.timestamp, end)?,
            attributes: self.attributes,
        })
    }
}

/// A run-time defined [`State`] of an [`Fsm`].
#[derive(Clone, Debug)]
pub struct RtState {
    pub name: String,
    pub usages: Vec<Usage>,
    pub span: SpanUnixNanoSec,
    pub attributes: Vec<Attribute>,
}

impl State for RtState {
    fn name(&self) -> &str {
        &self.name
    }
    fn span(&self) -> SpanUnixNanoSec {
        self.span
    }
    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes.iter()
    }
}

impl Using for RtState {
    fn usages(&self) -> impl Iterator<Item = (&Usage, SpanUnixNanoSec)> {
        self.usages.iter().map(|usage| (usage, self.span))
    }
}

/// Builder for run-time defined [`Fsm`]s with [`State`]s of type T.
pub struct RtFsmBuilder<T>
where
    T: Transition,
{
    id: Uuid,
    type_name: Option<String>,
    instance_name: Option<String>,
    transitions: OrderedStateTransitionCollector<T>,
}

impl<T> RtFsmBuilder<T>
where
    T: Transition,
{
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            type_name: None,
            instance_name: None,
            transitions: OrderedStateTransitionCollector::default(),
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
    pub fn extend(&mut self, states: impl IntoIterator<Item = T>) -> &mut Self {
        self.transitions.extend(states);
        self
    }
    pub fn push(&mut self, state: T) -> &mut Self {
        self.transitions.push(state);
        self
    }
}

impl RtFsmBuilder<RtTransition> {
    pub fn try_build(self) -> AnalyzerResult<RtFsm> {
        // Ensure we have >= 2 transitions
        let transitions: Vec<RtTransition> = self.transitions.try_into()?;
        // Ensure exit state per spec
        let last_name = &transitions.last().unwrap(/*len checked above*/).name;
        if last_name != "exit" {
            Err(AnalyzerError::Validation(format!(
                "final state of fsm {} is named {}, but must be named \"exit\"",
                self.id, last_name,
            )))
        } else {
            // Convert transitions into states. Ideally we'd use windows()
            // but transitions are consumed, so manually window over the
            // states.
            let mut sequence = Vec::with_capacity(transitions.len() - 1);
            let mut iter = transitions.into_iter();
            let mut current = iter.next().unwrap();
            for next in iter {
                let next_timestamp = next.timestamp();
                sequence.push(current.try_into_state(next_timestamp)?);
                current = next;
            }
            // For runtime-defined FSMs, there is no transition logic to check.

            Ok(RtFsm {
                id: self.id,
                type_name: self.type_name.ok_or_else(|| {
                    AnalyzerError::IncompleteEntity(format!("fsm {} has no type name", self.id))
                })?,
                instance_name: self.instance_name.ok_or_else(|| {
                    AnalyzerError::IncompleteEntity(format!("fsm {} has no instance name", self.id))
                })?,
                sequence,
            })
        }
    }
}

/// Run-time defined Finite-State-Machine
#[derive(Clone, Debug)]
pub struct RtFsm {
    id: Uuid,
    type_name: String,
    instance_name: String,
    sequence: Vec<RtState>,
}

impl Fsm for RtFsm {
    type StateType = RtState;
    fn states(&self) -> impl ExactSizeIterator<Item = &Self::StateType> {
        self.sequence.iter()
    }
    fn len(&self) -> usize {
        self.sequence.len()
    }
    fn state(&self, index: usize) -> Option<&Self::StateType> {
        self.sequence.get(index)
    }
}

#[cfg(test)]
impl RtFsm {
    pub fn try_new(
        id: Uuid,
        type_name: impl Into<String>,
        instance_name: impl Into<String>,
        transitions: impl IntoIterator<Item = RtTransition>,
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

impl Using for RtFsm {
    fn usages(&self) -> impl Iterator<Item = (&Usage, SpanUnixNanoSec)> {
        self.sequence.iter().flat_map(|state| state.usages())
    }
}

#[derive(Default)]
pub struct RtFsmsBuilder {
    fsms: HashMap<Uuid, RtFsmBuilder<RtTransition>>,
}

impl RtFsmsBuilder {
    pub fn push(&mut self, id: Uuid, transition: RtTransition) {
        self.fsms
            .entry(id)
            .or_insert_with(|| RtFsmBuilder::new(id))
            .push(transition);
    }

    pub fn try_build(self) -> AnalyzerResult<InMemoryFsms<RtFsm>> {
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
                RtTransition {
                    name: "a".to_string(),
                    usages: vec![],
                    timestamp: 1,
                    attributes: vec![],
                },
                RtTransition {
                    name: "b".to_string(),
                    usages: vec![],
                    timestamp: 2,
                    attributes: vec![],
                },
                RtTransition {
                    name: "c".to_string(),
                    usages: vec![],
                    timestamp: 3,
                    attributes: vec![],
                },
                RtTransition {
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
                [RtTransition {
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
