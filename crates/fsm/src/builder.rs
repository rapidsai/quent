// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{Entity, Identifier};

use crate::{ExitStates, Fsm, FsmError, Transition, check_entity};

/// Builds an [`Fsm`] for a specific [`Entity`], validating it on
/// [`FsmBuilder::build`].
pub struct FsmBuilder<'a> {
    entity: &'a Entity,
    initial_state: Identifier,
    transitions: Vec<Transition>,
    exit_from_states: ExitStates,
}

impl<'a> FsmBuilder<'a> {
    pub(crate) fn new(
        entity: &'a Entity,
        initial_state: Identifier,
        exit_state: Identifier,
    ) -> Self {
        Self {
            entity,
            initial_state,
            transitions: Vec::new(),
            exit_from_states: ExitStates {
                state: exit_state,
                others: Vec::new(),
            },
        }
    }

    /// Add a transition from `source` to `target`.
    pub fn transition(mut self, source: Identifier, target: Identifier) -> Self {
        self.transitions.push(Transition { source, target });
        self
    }

    /// Add another state the FSM may exit from (i.e. the FSM entity goes out of
    /// existence).
    pub fn exit_from(mut self, state: Identifier) -> Self {
        self.exit_from_states.others.push(state);
        self
    }

    /// Validate the FSM topology against the entity and return the FSM
    /// constraint, or return any violations found.
    pub fn build(self) -> Result<Fsm, FsmError> {
        let fsm = Fsm {
            initial_state: self.initial_state,
            transitions: self.transitions,
            exit_from_states: self.exit_from_states,
        };
        let mut errors = Vec::new();
        check_entity(self.entity, &fsm, &mut errors);
        match errors.len() {
            0 => Ok(fsm),
            1 => Err(errors.pop().unwrap()),
            _ => Err(FsmError::Multiple(errors)),
        }
    }
}
