use std::collections::HashSet;

use quent_attributes::Attribute;
use quent_time::{SpanNanoSec, TimeUnixNanoSec};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::{Entity, EntityRef, relation::Related, resource::Use};

/// Declaration of an FSM state
#[derive(TS, Clone, Debug, Serialize, Hash, PartialEq, Eq)]
pub struct StateDecl {
    pub name: String,
    // TODO(johanpel): attribute decls
    // TODO(johanpel): transition decls
}

impl StateDecl {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// Declaration of an FSM
#[derive(TS, Clone, Debug, Serialize)]
pub struct FsmTypeDecl {
    /// The unique type name of this FSM type
    pub name: String,
    /// Unordered set of states this FSM type
    pub states: HashSet<StateDecl>,
}

impl FsmTypeDecl {
    pub fn new(name: impl Into<String>, states: impl Iterator<Item = StateDecl>) -> Self {
        Self {
            name: name.into(),
            states: states.collect(),
        }
    }
    pub fn insert(&mut self, state_decl: StateDecl) {
        self.states.insert(state_decl);
    }
}

#[derive(TS, Clone, Debug, Serialize)]
pub struct State {
    // TODO(johanpel): consider deduplicating names by providing a state index
    // into an FSM vector
    pub name: String,
    pub uses: Vec<Use>,
    pub timestamp: TimeUnixNanoSec,
    pub attributes: Vec<Attribute>,
    pub relations: Vec<EntityRef>,
}

#[derive(TS, Clone, Default, Debug, Serialize)]
pub struct Fsm {
    pub id: Uuid,
    pub type_name: String,
    pub instance_name: Option<String>,
    pub state_sequence: Vec<State>,
}

#[derive(Debug)]
pub struct StateSpan<'a> {
    pub span: SpanNanoSec,
    pub state: &'a State,
}

impl Fsm {
    /// Return a state and a span for the index-th state.
    pub fn state_span(&self, index: usize) -> Option<StateSpan<'_>> {
        // If there are zero or one state transitions, a span cannot be created,
        // and this Fsm is violating the spec. Also check bounds.
        if self.state_sequence.len() < 2 || index >= self.state_sequence.len() - 1 {
            None
        } else {
            let start = self.state_sequence[index].timestamp;
            let end = self.state_sequence[index + 1].timestamp;
            Some(
                SpanNanoSec::try_new(start, end)
                    .map(|span| {
                        let state = &self.state_sequence[index];
                        StateSpan { span, state }
                    })
                    // TODO(johanpel): FSM may be broken when state sequence
                    // isn't following the "arrow of time". Perhaps introduce an
                    // FSM builder type that ensures built FSMs don't break this
                    // rule so we can safely unwrap the span constructed here.
                    .unwrap_or_else(|_| panic!("broken fsm: {self:#?}")),
            )
        }
    }

    pub fn state_spans(&self) -> impl ExactSizeIterator<Item = StateSpan<'_>> {
        (0..self.state_sequence.len().saturating_sub(1)).map(|index| {
            // Safety: through the saturating sub we can't go out of bounds,
            // even if this FSM is incomplete with zero or one transitions.
            self.state_span(index).unwrap()
        })
    }
}

impl Entity for Fsm {
    fn new(id: Uuid) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Related for State {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        self.relations.iter().cloned()
    }

    fn use_relations(&self) -> impl Iterator<Item = EntityRef> {
        self.uses.iter().map(|u| EntityRef::Resource(u.resource))
    }
}

impl Related for Fsm {
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
        let fsm = Fsm {
            id: Uuid::now_v7(),
            type_name: "test".to_string(),
            instance_name: Some("test_inst".to_string()),
            state_sequence: vec![
                State {
                    name: "a".to_string(),
                    uses: vec![],
                    timestamp: 1,
                    attributes: vec![],
                    relations: vec![],
                },
                State {
                    name: "b".to_string(),
                    uses: vec![],
                    timestamp: 2,
                    attributes: vec![],
                    relations: vec![],
                },
                State {
                    name: "c".to_string(),
                    uses: vec![],
                    timestamp: 3,
                    attributes: vec![],
                    relations: vec![],
                },
                State {
                    name: "exit".to_string(),
                    uses: vec![],
                    timestamp: 4,
                    attributes: vec![],
                    relations: vec![],
                },
            ],
        };

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
        let fsm = Fsm {
            id: Uuid::now_v7(),
            type_name: "test".to_string(),
            instance_name: Some("test_inst".to_string()),
            state_sequence: vec![State {
                name: "a".to_string(),
                uses: vec![],
                timestamp: 1,
                attributes: vec![],
                relations: vec![],
            }],
        };

        assert!(fsm.state_span(0).is_none());
        assert!(fsm.state_span(1).is_none());
        assert!(fsm.state_span(usize::MAX).is_none());
    }

    #[test]
    /// TODO(johanpel): solve panic, see [Fsm::state_span]
    #[should_panic]
    fn broken_fsm() {
        // Create an FSM with 2 state transitions, but the arrow of time is reversed
        let fsm = Fsm {
            id: Uuid::now_v7(),
            type_name: "test".to_string(),
            instance_name: Some("test_inst".to_string()),
            state_sequence: vec![
                State {
                    name: "a".to_string(),
                    uses: vec![],
                    timestamp: 2,
                    attributes: vec![],
                    relations: vec![],
                },
                State {
                    name: "b".to_string(),
                    uses: vec![],
                    timestamp: 1,
                    attributes: vec![],
                    relations: vec![],
                },
            ],
        };
        fsm.state_span(0);
    }
}
