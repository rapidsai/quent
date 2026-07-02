// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Quent built-in FSM constraint

use std::collections::{BTreeMap, HashSet};

use petgraph::{
    algo::tarjan_scc,
    graphmap::DiGraphMap,
    visit::{Bfs, Reversed, Walker},
};
use quent_constraints::{Constraint, utils::bullet_list};
use quent_schema::{
    Cardinality, Entity, Identifier,
    visitor::{Cursor, Element, Visitor},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod builder;

pub use builder::FsmBuilder;

/// A directed transition between two named states in an [`Fsm`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Transition {
    /// The name of the source state.
    source: Identifier,
    /// The name of the target state.
    target: Identifier,
}

impl Transition {
    /// The name of the source state.
    pub fn source(&self) -> &Identifier {
        &self.source
    }

    /// The name of the target state.
    pub fn target(&self) -> &Identifier {
        &self.target
    }
}

/// A non-empty set of states from which an `Fsm` can transition out of
/// existence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ExitStates {
    state: Identifier,
    others: Vec<Identifier>,
}

/// The state-transition topology of a finite state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fsm {
    /// The name of the initial state this FSM transitions into when it comes
    /// into existence.
    initial_state: Identifier,
    /// The possible transitions of this FSM.
    transitions: Vec<Transition>,
    /// The states (at least one) from which this FSM can exit to go out of
    /// existence.
    exit_from_states: ExitStates,
}

impl Fsm {
    /// Return a builder for an [`Fsm`] constraint over `entity`, beginning in
    /// `initial_state` and able to exit from `exit_state`. Further exit states
    /// can be added on the builder.
    pub fn builder(
        entity: &Entity,
        initial_state: Identifier,
        exit_state: Identifier,
    ) -> FsmBuilder<'_> {
        FsmBuilder::new(entity, initial_state, exit_state)
    }

    /// The name of the initial state this FSM transitions into when it comes
    /// into existence.
    pub fn initial_state(&self) -> &Identifier {
        &self.initial_state
    }

    /// The transitions of this FSM between states.
    ///
    /// This excludes exit transitions. Obtain these through
    /// [`Self::exit_from_states`].
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// The states from which this FSM can exit to go out of existence.
    pub fn exit_from_states(&self) -> impl Iterator<Item = &Identifier> {
        std::iter::once(&self.exit_from_states.state).chain(self.exit_from_states.others.iter())
    }
}

/// Constrains the order of an entity's events by a Finite-State-Machine
/// topology.
///
/// Through this constraint, the behavior of an entity can be modeled as an FSM.
/// The event order is restricted by a certain topology of "states" and
/// "transitions" between those states, where events represent the moment in
/// time the FSM transitions into a state with the name of the event.
///
/// Modeling entities as FSMs is useful to trace a specific restricted lifecycle
/// of the entity. The lifecycle has a single initial state where the FSM entity
/// comes into existence, and a set of states from which it may go out of
/// existence through an "exit" transition. The topology must be formed such
/// that every state is reachable from the initial state, and from every state
/// there exists a sequence of states that leads to an exit transition.
///
/// The moment an entity modeled as an FSM in the client code transitions
/// between states, the transition event is to be emitted. At that time, both
/// trigger conditions and state outputs can be captured in the event's
/// attributes. Where applicable, if users desire to capture changes to an FSM's
/// outputs as a function of its inputs without advancing to a different state,
/// this can be modeled as a self-transition that updates those attributes.
///
/// Be aware that Quent's concept of an FSM is a strict subset of what can
/// typically be expressed in full-fledged finite-state-automata theory.
///
/// ## Requirements
///
/// For every entity carrying this constraint:
///
/// 1. No event in the entity may be named `exit` (case-insensitively).
/// 2. Every state named by the FSM corresponds to an event name in the entity.
/// 3. Every event in the entity appears as a state in the FSM.
/// 4. Every state is reachable from the initial state.
/// 5. An exit transition is reachable from every state.
/// 6. A state on a cycle has [`Cardinality::Multi`], otherwise
///    [`Cardinality::Once`].
/// 7. At least one exit transition exists (enforced by [`ExitStates`]).
#[derive(Default)]
pub struct FsmConstraint {
    errors: Vec<FsmError>,
}

impl Visitor for FsmConstraint {
    type Output = Result<(), FsmError>;

    fn visit(&mut self, cursor: &Cursor) {
        let Element::Entity(entity) = cursor.current() else {
            return;
        };
        let Some(constraint) = entity.annotations().constraint(FsmConstraint::NAME) else {
            return;
        };
        let raw = match constraint.data() {
            Some(s) => s,
            None => {
                self.errors.push(FsmError::InvalidData {
                    entity: entity.name().clone(),
                    message: "constraint data is missing".to_string(),
                });
                return;
            }
        };
        let fsm = match serde_json::from_str::<Fsm>(raw) {
            Ok(f) => f,
            Err(e) => {
                self.errors.push(FsmError::InvalidData {
                    entity: entity.name().clone(),
                    message: format!("failed to decode fsm: {e}"),
                });
                return;
            }
        };
        check_entity(entity, &fsm, &mut self.errors);
    }

    fn finish(self) -> Self::Output {
        match self.errors.len() {
            0 => Ok(()),
            1 => Err(self.errors.into_iter().next().unwrap()),
            _ => Err(FsmError::Multiple(self.errors)),
        }
    }
}

impl Constraint for FsmConstraint {
    const NAME: &'static str = "quent.fsm.v1";
}

pub(crate) fn check_entity(entity: &Entity, fsm: &Fsm, errors: &mut Vec<FsmError>) {
    // Requirement 1: no event may be named "exit".
    for event in entity.events() {
        if event.name().to_ascii_lowercase() == "exit" {
            errors.push(FsmError::ReservedStateName {
                entity: entity.name().clone(),
                name: "exit",
            });
        }
    }

    let event_names: HashSet<&Identifier> = entity.events().map(|e| e.name()).collect();
    let cardinality_by_event: BTreeMap<&Identifier, Cardinality> = entity
        .events()
        .map(|e| (e.name(), e.cardinality()))
        .collect();

    // Gather every state named
    let states: HashSet<&Identifier> = std::iter::once(&fsm.initial_state)
        .chain(fsm.transitions.iter().flat_map(|t| [&t.source, &t.target]))
        .chain(fsm.exit_from_states())
        .collect();

    // Requirement 2: every state name corresponds to an entity event name.
    for &state in states.difference(&event_names) {
        errors.push(FsmError::UnknownState {
            entity: entity.name().clone(),
            state: state.clone(),
        });
    }

    // Requirement 3: every entity event appears as a state.
    for &event in event_names.difference(&states) {
        errors.push(FsmError::UncoveredEvent {
            entity: entity.name().clone(),
            event: event.clone(),
        });
    }

    // Graph of states + transitions
    let graph: DiGraphMap<GraphNode, ()> =
        std::iter::once((GraphNode::Init, GraphNode::Named(&fsm.initial_state), ()))
            .chain(
                fsm.transitions
                    .iter()
                    .map(|t| (GraphNode::Named(&t.source), GraphNode::Named(&t.target), ())),
            )
            .chain(
                fsm.exit_from_states()
                    .map(|x| (GraphNode::Named(x), GraphNode::Exit, ())),
            )
            .collect();

    // Requirement 4: every state is reachable from the initial state.
    let reachable_from_init: HashSet<GraphNode> =
        Bfs::new(&graph, GraphNode::Init).iter(&graph).collect();
    for &name in &states {
        if !reachable_from_init.contains(&GraphNode::Named(name)) {
            errors.push(FsmError::UnreachableFromInit {
                entity: entity.name().clone(),
                state: name.clone(),
            });
        }
    }

    // Requirement 5: an exit transition is reachable from every state.
    let reversed = Reversed(&graph);
    let reaches_exit: HashSet<GraphNode> =
        Bfs::new(reversed, GraphNode::Exit).iter(reversed).collect();
    for &name in &states {
        if !reaches_exit.contains(&GraphNode::Named(name)) {
            errors.push(FsmError::CannotReachExit {
                entity: entity.name().clone(),
                state: name.clone(),
            });
        }
    }

    // Requirement 6: a state on a cycle is Multi, otherwise Once.
    let on_cycle = find_cyclic(&graph, fsm);
    for &name in &states {
        let expected_cardinality = if on_cycle.contains(name) {
            Cardinality::Multi
        } else {
            Cardinality::Once
        };
        let Some(actual) = cardinality_by_event.get(name) else {
            continue;
        };
        if *actual != expected_cardinality {
            errors.push(FsmError::CardinalityMismatch {
                entity: entity.name().clone(),
                state: name.clone(),
                expected: expected_cardinality,
                found: *actual,
            });
        }
    }
}

/// Compute the states that lie on a cycle in the transition graph.
fn find_cyclic<'a>(graph: &DiGraphMap<GraphNode<'a>, ()>, fsm: &'a Fsm) -> HashSet<&'a Identifier> {
    let mut on_cycle = HashSet::new();
    // A node is on a cycle if it sits in a strongly connected component of more
    // than one node.
    for scc in tarjan_scc(graph) {
        if scc.len() > 1 {
            for node in scc {
                if let GraphNode::Named(name) = node {
                    on_cycle.insert(name);
                }
            }
        }
    }
    // It also sits on a cycle if it has a self-loop, but `tarjan_scc` does not
    // report those as cyclic, so do them separately:
    for t in &fsm.transitions {
        if t.source == t.target {
            on_cycle.insert(&t.source);
        }
    }
    on_cycle
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum GraphNode<'a> {
    Init,
    Exit,
    Named(&'a Identifier),
}

#[derive(Debug, Error)]
pub enum FsmError {
    #[error("entity \"{entity}\" fsm: {message}")]
    InvalidData { entity: Identifier, message: String },
    #[error("entity \"{entity}\" fsm: \"{name}\" is a reserved state name")]
    ReservedStateName {
        entity: Identifier,
        name: &'static str,
    },
    #[error("entity \"{entity}\" fsm: state \"{state}\" is unreachable from the initial state")]
    UnreachableFromInit {
        entity: Identifier,
        state: Identifier,
    },
    #[error("entity \"{entity}\" fsm: state \"{state}\" cannot reach any exit transition")]
    CannotReachExit {
        entity: Identifier,
        state: Identifier,
    },
    #[error("entity \"{entity}\" fsm: state \"{state}\" does not match any event")]
    UnknownState {
        entity: Identifier,
        state: Identifier,
    },
    #[error("entity \"{entity}\" fsm: event \"{event}\" does not appear as a state")]
    UncoveredEvent {
        entity: Identifier,
        event: Identifier,
    },
    #[error(
        "entity \"{entity}\" fsm: state \"{state}\" expects cardinality {expected:?}, but event has {found:?}"
    )]
    CardinalityMismatch {
        entity: Identifier,
        state: Identifier,
        expected: Cardinality,
        found: Cardinality,
    },
    #[error("multiple fsm violations:\n{}", bullet_list(.0))]
    Multiple(Vec<FsmError>),
}
