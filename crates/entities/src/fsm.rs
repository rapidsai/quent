use std::collections::HashSet;

use quent_attributes::Attribute;
use quent_time::{SpanNanoSec, TimeUnixNanoSec, span::SpanUnixNanoSec};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    Entity, EntityRef, IncompleteEntity, Lifetime, Result, error::EntityError, relation::Related,
    resource::Use,
};

/// Trait to express something is an FSM state.
pub trait State {
    /// Return the unique name of this state.
    fn name(&self) -> &str;

    /// Return an iterator over the resources this state is using.
    fn uses(&self) -> impl Iterator<Item = &Use>;

    /// Return the timestamp when the FSM transitioned into this state.
    fn timestamp(&self) -> TimeUnixNanoSec;

    /// Return an iterator over arbitrary key-value attributes associated with this state.
    fn attributes(&self) -> impl Iterator<Item = &Attribute>;

    /// Return an iterator over references to other entities related to this state.
    fn relations(&self) -> impl Iterator<Item = EntityRef>;
}

/// Trait to express something is an FSM.
//
// Surpress this, because if something is an FSM, it always has two states, and
// it is never empty.
#[allow(clippy::len_without_is_empty)]
pub trait Fsm {
    /// The state type of this FSM.
    type State: State;

    /// Return the ID of this FSM.
    fn id(&self) -> Uuid;

    /// Return the type name of this FSM.
    fn type_name(&self) -> &str;

    /// Return the name of this FSM instance, if any.
    fn instance_name(&self) -> Option<&str>;

    /// Return the number of states
    fn len(&self) -> usize;

    /// Return a reference to the index-th state, if the index is not out of
    /// bounds.
    fn index(&self, index: usize) -> Option<&Self::State>;

    /// Return a state and its time span for the given index.
    fn state_span(&self, index: usize) -> Option<StateSpan<'_, Self::State>> {
        // If there are zero or one state transitions, a span cannot be created,
        // and this Fsm is violating the spec. Also check bounds.
        if self.len() < 2 || index >= self.len() - 1 {
            None
        } else {
            let start = self.index(index).unwrap().timestamp();
            let end = self.index(index + 1).unwrap().timestamp();
            Some(
                SpanNanoSec::try_new(start, end)
                    .map(|span| {
                        let state = self.index(index).unwrap();
                        StateSpan { span, state }
                    })
                    // This should never happen for properly constructed FSMs.
                    // If it does, this is a bug inside this analyzer code, so
                    // panic to make sure I get publicly shamed and never do it
                    // again.
                    .unwrap_or_else(|_| {
                        panic!(
                            "causality violation in fsm {} with timestamp sequence {:?}",
                            self.id(),
                            self.states().map(|s| s.timestamp()).collect::<Vec<_>>()
                        )
                    }),
            )
        }
    }

    /// Return an iterator over all states with their time spans.
    fn state_spans(&self) -> impl ExactSizeIterator<Item = StateSpan<'_, Self::State>> {
        (0..self.len().saturating_sub(1)).map(|index| {
            // Safety: through the saturating sub we can't go out of bounds,
            // even if this FSM is incomplete with zero or one transitions.
            self.state_span(index).unwrap()
        })
    }

    /// Return an iterator over all state transitions.
    fn states(&self) -> impl ExactSizeIterator<Item = &Self::State>;
}

impl<U: Fsm> Entity for U {
    fn id(&self) -> Uuid {
        Fsm::id(self)
    }
    fn lifetime(&self) -> Lifetime {
        Lifetime::Span(
            SpanUnixNanoSec::try_new(
                self.index(0).unwrap().timestamp(),
                self.index(self.len() - 1).unwrap().timestamp(),
            )
            .unwrap(),
        )
    }
}

/// Declaration of an FSM state.
#[derive(TS, Clone, Debug, Serialize, Hash, PartialEq, Eq)]
pub struct DynamicFsmStateDecl {
    pub name: String,
    // TODO(johanpel): attribute decls
    // TODO(johanpel): transition decls
}

impl DynamicFsmStateDecl {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// Declaration of an FSM
#[derive(TS, Clone, Debug, Serialize)]
pub struct DynamicFsmTypeDecl {
    /// The unique type name of this FSM type
    pub name: String,
    /// Unordered set of states this FSM type
    pub states: HashSet<DynamicFsmStateDecl>,
}

impl DynamicFsmTypeDecl {
    pub fn new(name: impl Into<String>, states: impl Iterator<Item = DynamicFsmStateDecl>) -> Self {
        Self {
            name: name.into(),
            states: states.collect(),
        }
    }
    pub fn insert(&mut self, state_decl: DynamicFsmStateDecl) {
        self.states.insert(state_decl);
    }
}

/// A run-time defined [`State`] of an [`Fsm`].
#[derive(TS, Clone, Debug, Serialize)]
pub struct DynamicState {
    // TODO(johanpel): consider deduplicating names by providing a state index
    // into an FSM vector
    pub name: String,
    pub uses: Vec<Use>,
    pub timestamp: TimeUnixNanoSec,
    pub attributes: Vec<Attribute>,
    pub relations: Vec<EntityRef>,
}

impl State for DynamicState {
    fn name(&self) -> &str {
        &self.name
    }
    fn uses(&self) -> impl Iterator<Item = &Use> {
        self.uses.iter()
    }
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attributes.iter()
    }
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        self.relations.iter().cloned()
    }
}

impl Related for DynamicState {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        self.relations.iter().cloned()
    }
    fn use_relations(&self) -> impl Iterator<Item = EntityRef> {
        self.uses.iter().map(|u| EntityRef::Resource(u.resource))
    }
}

pub struct StateSequenceBuilder<T>
where
    T: State,
{
    pub sequence: Vec<T>,
}

impl<T> Default for StateSequenceBuilder<T>
where
    T: State,
{
    fn default() -> Self {
        Self {
            sequence: Default::default(),
        }
    }
}

impl<T> StateSequenceBuilder<T>
where
    T: State,
{
    pub fn with_state(&mut self, state: T) -> &mut Self {
        // The common case is for events to arrive in order, so a simple Vec
        // will suffice to make the vast majority of ordered insertions fast.
        if let Some(last) = self.sequence.last()
            && last.timestamp() <= state.timestamp()
        {
            self.sequence.push(state);
        } else {
            let pos = self
                .sequence
                .binary_search_by(|s| s.timestamp().cmp(&state.timestamp()))
                .unwrap_or_else(|i| i);
            self.sequence.insert(pos, state);
        }
        self
    }

    pub fn push_state(&mut self, state: T) {
        self.with_state(state);
    }

    pub fn with_states(&mut self, states: impl IntoIterator<Item = T>) -> &mut Self {
        states
            .into_iter()
            .fold(self, |builder, state| builder.with_state(state))
    }
}

/// Builder for [`Fsm`]
pub struct FsmBuilder<T>
where
    T: State,
{
    id: Uuid,
    type_name: Option<String>,
    instance_name: Option<String>,
    states: StateSequenceBuilder<T>,
}

impl<T> IncompleteEntity for FsmBuilder<T>
where
    T: State,
{
    fn new(id: Uuid) -> Self {
        Self {
            id,
            type_name: None,
            instance_name: None,
            states: StateSequenceBuilder { sequence: vec![] },
        }
    }
}

impl<T> FsmBuilder<T>
where
    T: State,
{
    pub fn with_type_name(&mut self, type_name: impl Into<String>) -> &mut Self {
        self.type_name = Some(type_name.into());
        self
    }
    pub fn with_instance_name(&mut self, instance_name: Option<String>) -> &mut Self {
        self.instance_name = instance_name;
        self
    }
    pub fn with_states(&mut self, states: impl IntoIterator<Item = T>) -> &mut Self {
        self.states.with_states(states);
        self
    }

    pub fn push_state(&mut self, state: T) {
        self.states.push_state(state);
    }
}

impl FsmBuilder<DynamicState> {
    pub fn try_build(self) -> Result<DynamicFsm> {
        // TODO(johanpel): state transition validation logic goes here.
        // Ensure there are at least two states:
        if self.states.sequence.len() < 2 {
            Err(EntityError::IncompleteFsm(format!(
                "fsm {} has {} states, but must have >= 2 states",
                self.id,
                self.states.sequence.len()
            )))
        } else {
            Ok(DynamicFsm {
                id: self.id,
                type_name: self.type_name.ok_or(EntityError::IncompleteFsm(format!(
                    "FSM {} requires a type name",
                    self.id
                )))?,
                instance_name: self.instance_name,
                state_sequence: self.states.sequence,
            })
        }
    }
}

#[derive(Debug)]
pub struct StateSpan<'a, S: State> {
    pub span: SpanNanoSec,
    pub state: &'a S,
}

/// Run-time defined Finite-State-Machine
#[derive(TS, Clone, Debug, Serialize)]
pub struct DynamicFsm {
    id: Uuid,
    type_name: String,
    instance_name: Option<String>,
    state_sequence: Vec<DynamicState>,
}

impl Fsm for DynamicFsm {
    type State = DynamicState;

    fn type_name(&self) -> &str {
        &self.type_name
    }
    fn instance_name(&self) -> Option<&str> {
        self.instance_name.as_deref()
    }
    fn states(&self) -> impl ExactSizeIterator<Item = &Self::State> {
        self.state_sequence.iter()
    }
    fn id(&self) -> Uuid {
        self.id
    }
    fn len(&self) -> usize {
        self.state_sequence.len()
    }
    fn index(&self, index: usize) -> Option<&Self::State> {
        self.state_sequence.get(index)
    }
}

impl DynamicFsm {
    pub fn try_new(
        id: Uuid,
        type_name: impl Into<String>,
        instance_name: Option<String>,
        states: impl IntoIterator<Item = DynamicState>,
    ) -> Result<DynamicFsm> {
        let mut bld = FsmBuilder::new(id);
        bld.with_type_name(type_name)
            .with_instance_name(instance_name)
            .with_states(states);
        bld.try_build()
    }
}

impl Related for DynamicFsm {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        self.state_sequence
            .iter()
            .flat_map(|state| state.relations.iter())
            .cloned()
    }

    fn use_relations(&self) -> impl Iterator<Item = EntityRef> {
        self.state_sequence
            .iter()
            .flat_map(|state| state.use_relations())
            .collect::<HashSet<_>>()
            .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_span() {
        // Create an FSM with 3 state transitions plus an exit state
        let fsm = DynamicFsm::try_new(
            Uuid::now_v7(),
            "test",
            None,
            [
                DynamicState {
                    name: "a".to_string(),
                    uses: vec![],
                    timestamp: 1,
                    attributes: vec![],
                    relations: vec![],
                },
                DynamicState {
                    name: "b".to_string(),
                    uses: vec![],
                    timestamp: 2,
                    attributes: vec![],
                    relations: vec![],
                },
                DynamicState {
                    name: "c".to_string(),
                    uses: vec![],
                    timestamp: 3,
                    attributes: vec![],
                    relations: vec![],
                },
                DynamicState {
                    name: "exit".to_string(),
                    uses: vec![],
                    timestamp: 4,
                    attributes: vec![],
                    relations: vec![],
                },
            ],
        )
        .unwrap();

        let span = fsm.state_span(0).unwrap();
        assert_eq!(span.state.name, "a");
        assert_eq!(span.span.start(), 1);
        assert_eq!(span.span.end(), 2);

        let span = fsm.state_span(1).unwrap();
        assert_eq!(span.state.name, "b");
        assert_eq!(span.span.start(), 2);
        assert_eq!(span.span.end(), 3);

        let span = fsm.state_span(2).unwrap();
        assert_eq!(span.state.name, "c");
        assert_eq!(span.span.start(), 3);
        assert_eq!(span.span.end(), 4);

        assert!(fsm.state_span(3).is_none());
        assert!(fsm.state_span(usize::MAX).is_none());

        assert_eq!(fsm.state_spans().len(), 3);
        for (index, state_span) in fsm.state_spans().enumerate() {
            assert_eq!(state_span.span.start(), 1 + index as u64);
            assert_eq!(state_span.span.end(), 2 + index as u64);
            assert_eq!(state_span.state.name, ["a", "b", "c"][index]);
        }
    }

    #[test]
    fn incomplete_fsm() {
        // Create an FSM with 1 state transition. No span can be derived from it.
        assert!(
            DynamicFsm::try_new(
                Uuid::now_v7(),
                "test",
                None,
                [DynamicState {
                    name: "a".to_string(),
                    uses: vec![],
                    timestamp: 1,
                    attributes: vec![],
                    relations: vec![],
                }],
            )
            .is_err()
        );
    }
}
