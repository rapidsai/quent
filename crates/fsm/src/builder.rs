// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use quent_constraints::Constraint;
use quent_schema::builder::{AnnotationsBuilder, BuilderError, EntityBuilder, EventBuilder};
use quent_schema::{Annotations, Cardinality, Entity, Field, Identifier, Path};
use thiserror::Error;

use crate::{Fsm, FsmConstraint, FsmError, Transition, check_entity};

/// A declared state of an FSM entity.
pub struct StateDecl {
    /// State name, used verbatim as the state event's name.
    pub name: Identifier,
    /// Fields of the state event.
    pub attributes: Vec<Field>,
    /// States this state transitions to. An empty list makes this a final state.
    pub to: Vec<Identifier>,
    /// Whether the FSM begins in this state.
    pub initial: bool,
}

/// Builds an FSM [`Entity`] from its states: each state becomes one event whose
/// cardinality is derived from the topology, plus the FSM constraint.
///
/// [`Self::build`] validates the topology with the same checks as
/// [`crate::FsmConstraint`], so a built entity is always valid.
pub struct FsmEntityBuilder {
    path: Path,
    annotations: AnnotationsBuilder,
    states: Vec<StateDecl>,
}

/// A problem that prevents building a valid FSM entity.
#[derive(Debug, Error)]
pub enum FsmEntityBuilderError {
    #[error("no state is marked as the initial state")]
    NoInitialState,
    #[error("more than one state is marked as the initial state")]
    MultipleInitialStates(Vec<Identifier>),
    #[error("duplicate state `{0}`")]
    DuplicateState(Identifier),
    /// The generated schema element is invalid.
    #[error(transparent)]
    Build(#[from] BuilderError),
    /// The FSM topology failed to serialize to its constraint payload.
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
    /// The FSM topology is invalid (unreachable state, no path to a final state, ...).
    #[error(transparent)]
    Invalid(#[from] FsmError),
}

impl FsmEntityBuilder {
    /// Begin an FSM entity at `path`.
    pub fn new(path: impl Into<Path>) -> Self {
        Self {
            path: path.into(),
            annotations: AnnotationsBuilder::new(),
            states: Vec::new(),
        }
    }

    /// Set the entity's annotations, replacing any set so far, and return the
    /// builder for chaining. The FSM constraint is added to them on
    /// [`Self::build`].
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Add a state.
    pub fn with_state(mut self, state: StateDecl) -> Self {
        self.states.push(state);
        self
    }

    /// Add several states.
    pub fn with_states(mut self, states: impl IntoIterator<Item = StateDecl>) -> Self {
        self.states.extend(states);
        self
    }

    /// Assemble and validate the entity: derive each state event's cardinality
    /// from the topology, attach the events and the FSM constraint, then check
    /// the topology.
    ///
    /// # Errors
    ///
    /// Returns [`FsmEntityBuilderError`] if a state name is declared twice,
    /// there is not exactly one initial state, a state has a duplicate
    /// attribute name, the topology fails to serialize, or the topology is
    /// invalid.
    pub fn build(self) -> Result<Entity, FsmEntityBuilderError> {
        let Self {
            path,
            mut annotations,
            states,
        } = self;

        // Reject duplicate state names up front, before the structural checks
        // could misattribute them (e.g. two states both marked initial).
        let mut seen = HashSet::new();
        for state in &states {
            if !seen.insert(&state.name) {
                return Err(FsmEntityBuilderError::DuplicateState(state.name.clone()));
            }
        }

        let initials: Vec<Identifier> = states
            .iter()
            .filter(|s| s.initial)
            .map(|s| s.name.clone())
            .collect();
        let initial = match initials.as_slice() {
            [initial] => initial.clone(),
            [] => return Err(FsmEntityBuilderError::NoInitialState),
            _ => return Err(FsmEntityBuilderError::MultipleInitialStates(initials)),
        };

        let transitions = states
            .iter()
            .flat_map(|state| {
                let source = state.name.clone();
                state
                    .to
                    .iter()
                    .map(move |target| Transition::new(source.clone(), target.clone()))
            })
            .collect();
        let fsm = Fsm::new(initial, transitions);

        let mut entity = EntityBuilder::new(path);
        for state in states {
            let cardinality = fsm.cardinality(&state.name).unwrap_or(Cardinality::Once);
            let event = EventBuilder::new(state.name, cardinality)
                .with_fields(state.attributes)
                .build()?;
            entity = entity.with_event(event);
        }

        annotations =
            annotations.with_constraint(FsmConstraint::NAME, Some(fsm.constraint_data()?));
        let entity = entity.with_annotations(annotations.build()?).build()?;

        // Validate the full topology now, the same checks the constraint runs
        // during schema validation, so a built entity is always valid.
        let mut topology_errors = Vec::new();
        check_entity(&entity, &fsm, &mut topology_errors);
        if let Some(error) = topology_errors.into_iter().next() {
            Err(FsmEntityBuilderError::Invalid(error))
        } else {
            Ok(entity)
        }
    }
}
