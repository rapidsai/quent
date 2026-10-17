// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of instrumentation handles for FSM entities.

use proc_macro2::TokenStream;
use quent_constraints::Constraint as _;
use quent_fsm::Fsm;
use quent_schema::{Entity, Identifier, Schema};
use quote::quote;

use super::GeneratedHandle;
use crate::{GenerateError, runtime::event_ident};

mod dynamic;
mod typestate;

/// The maximum number of declared FSM states, leaving zero for the new state.
pub(crate) const MAX_FSM_STATES: usize = u8::MAX as usize;

const RESERVED_HANDLE_METHOD_NAMES: [&str; 9] = [
    "state",
    "state_name",
    "try_into",
    "into_dynamic",
    "id",
    "as_entity_ref",
    "as_entity_ref_with",
    "as_any_entity_ref",
    "as_any_entity_ref_with",
];

fn reserved_handle_method(state: &Identifier) -> Option<&'static str> {
    let method = crate::common::to_case(state, convert_case::Case::Snake);
    RESERVED_HANDLE_METHOD_NAMES
        .into_iter()
        .find(|reserved| *reserved == method)
}

pub(super) fn entity_handle(entity: &Entity) -> Result<Option<GeneratedHandle>, GenerateError> {
    let Some(fsm) = Fsm::try_from_entity(entity)? else {
        return Ok(None);
    };
    let state_count = entity.events().count();
    if state_count > MAX_FSM_STATES {
        return Err(GenerateError::TooManyFsmStates {
            entity: entity.path().clone(),
            count: state_count,
        });
    }
    for state in entity.events() {
        if let Some(method) = reserved_handle_method(state.name()) {
            return Err(GenerateError::ReservedFsmHandleMethod {
                entity: entity.path().clone(),
                state: state.name().clone(),
                method,
            });
        }
    }
    let handle_ty = typestate::handle_type_path(entity);
    let fsm_event = fsm_event_impl(entity);
    let typestate = typestate::entity_impl(entity, &fsm)?;
    let dynamic = dynamic::entity_impl(entity, &fsm)?;
    Ok(Some(GeneratedHandle {
        tokens: quote! {
            #fsm_event
            #typestate
            #dynamic
        },
        associated_type: quote! { #handle_ty<Self> },
    }))
}

pub(super) fn handle_type(schema: &Schema) -> TokenStream {
    if !schema.entities().any(|entity| {
        entity
            .annotations()
            .has_constraint(quent_fsm::FsmConstraint::NAME)
    }) {
        return TokenStream::new();
    }
    let typestate = typestate::handle_type(schema);
    let dynamic = dynamic::handle_type(schema);
    quote! {
        #typestate
        #dynamic
    }
}

fn fsm_event_impl(entity: &Entity) -> TokenStream {
    let event_ty = event_ident(entity);
    let sequence_arms = entity.events().map(|event| {
        let variant = crate::common::raw_ident(crate::common::to_case(
            event.name(),
            convert_case::Case::Pascal,
        ));
        quote! { Self::#variant { seq, .. } => *seq = sequence }
    });
    quote! {
        impl ::quent_instrumentation::FsmEvent for #event_ty {
            fn set_sequence(&mut self, sequence: u16) {
                match self {
                    #(#sequence_arms),*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quent_fsm::{FsmEntityBuilder, StateDecl};
    use quent_schema::{DataType, Field, Identifier};

    use super::*;
    use crate::{pretty, runtime::handle::MAX_ONCE_EVENTS};

    fn state(name: &str, to: &[&str], initial: bool, fields: Vec<Field>) -> StateDecl {
        StateDecl {
            name: Identifier::try_new(name).unwrap(),
            attributes: fields,
            to: to
                .iter()
                .map(|target| Identifier::try_new(*target).unwrap())
                .collect(),
            initial,
        }
    }

    fn source(entity: &Entity) -> String {
        pretty(entity_handle(entity).unwrap().unwrap().tokens)
    }

    #[test]
    fn generates_consuming_linear_transitions_without_sequence_parameters() {
        let entity = FsmEntityBuilder::new("Query".parse::<quent_schema::Path>().unwrap())
            .with_states([
                state(
                    "submitted",
                    &["running"],
                    true,
                    vec![Field::new(
                        Identifier::try_new("text").unwrap(),
                        DataType::String,
                        Default::default(),
                    )],
                ),
                state("running", &["ready"], false, vec![]),
                state("ready", &[], false, vec![]),
            ])
            .build()
            .unwrap();

        let source = source(&entity);

        assert!(source.contains("pub mod query_state"));
        assert!(source.contains("pub enum Submitted"));
        assert!(source.contains("impl FsmHandle<Query>"));
        assert!(source.contains("pub fn submitted(self, text: String)"));
        assert!(source.contains("-> FsmHandle<Query, query_state::Submitted>"));
        assert!(source.contains("impl FsmHandle<Query, query_state::Submitted>"));
        assert!(source.contains(".transition(QueryEvent::Submitted"));
        assert!(source.contains("seq: 0"));
        assert!(!source.contains("seq: u16"));
        assert!(source.contains("pub enum QueryDynamicState"));
        assert!(
            source.contains(
                "impl ::quent_instrumentation::FsmState<Query> for query_state::Submitted"
            )
        );
        assert!(source.contains("impl DynamicFsmHandle<Query>"));
        assert!(source.contains("pub fn into_dynamic(self) -> DynamicFsmHandle<Query>"));
        assert!(source.contains("pub fn try_into<S>("));
        assert!(source.contains("S: ::quent_instrumentation::FsmState<Query>"));
        assert!(source.contains("pub fn submitted(\n        &mut self"));
        assert!(source.contains("self.inner.transition_mut("));
        assert!(source.contains("impl FsmHandle<Query, query_state::Ready>"));
    }

    #[test]
    fn generates_branching_cycles_self_transitions_and_namespaced_states() {
        let entity = FsmEntityBuilder::new("jobs::Query".parse::<quent_schema::Path>().unwrap())
            .with_states([
                state("submitted", &["running", "failed"], true, vec![]),
                state(
                    "running",
                    &["running", "running", "ready", "failed"],
                    false,
                    vec![],
                ),
                state("ready", &[], false, vec![]),
                state("failed", &[], false, vec![]),
            ])
            .build()
            .unwrap();

        let source = source(&entity);

        assert!(source.contains("impl super::FsmHandle<Query>"));
        assert!(source.contains("impl super::FsmHandle<Query, query_state::Running>"));
        assert!(source.contains("-> super::FsmHandle<Query, query_state::Running>"));
        assert_eq!(source.matches("pub fn running(").count(), 3);
        assert!(source.contains("pub fn ready("));
        assert!(source.contains("pub fn failed("));
        assert!(source.contains("impl super::FsmHandle<Query, query_state::Ready>"));
        assert!(source.contains("impl super::FsmHandle<Query, query_state::Failed>"));
        assert!(source.contains("impl super::DynamicFsmHandle<Query>"));
    }

    #[test]
    fn fsm_entities_bypass_the_once_event_limit() {
        let states = (0..=MAX_ONCE_EVENTS)
            .map(|index| {
                let name = format!("state_{index}");
                let next = (index < MAX_ONCE_EVENTS).then(|| format!("state_{}", index + 1));
                StateDecl {
                    name: Identifier::try_new(name).unwrap(),
                    attributes: vec![],
                    to: next
                        .into_iter()
                        .map(|target| Identifier::try_new(target).unwrap())
                        .collect(),
                    initial: index == 0,
                }
            })
            .collect::<Vec<_>>();
        let entity = FsmEntityBuilder::new("Large".parse::<quent_schema::Path>().unwrap())
            .with_states(states)
            .build()
            .unwrap();

        assert!(matches!(entity_handle(&entity), Ok(Some(_))));
    }

    #[test]
    fn rejects_more_states_than_the_dynamic_representation_supports() {
        let states = (0..=MAX_FSM_STATES)
            .map(|index| {
                let name = format!("state_{index}");
                let next = (index < MAX_FSM_STATES).then(|| format!("state_{}", index + 1));
                StateDecl {
                    name: Identifier::try_new(name).unwrap(),
                    attributes: vec![],
                    to: next
                        .into_iter()
                        .map(|target| Identifier::try_new(target).unwrap())
                        .collect(),
                    initial: index == 0,
                }
            })
            .collect::<Vec<_>>();
        let entity = FsmEntityBuilder::new("Large".parse::<quent_schema::Path>().unwrap())
            .with_states(states)
            .build()
            .unwrap();

        assert!(matches!(
            entity_handle(&entity),
            Err(GenerateError::TooManyFsmStates { count, .. }) if count == MAX_FSM_STATES + 1
        ));
    }

    #[test]
    fn rejects_state_names_reserved_by_generated_handles() {
        for (state_name, expected_method) in [
            ("State", "state"),
            ("StateName", "state_name"),
            ("tryInto", "try_into"),
            ("into_dynamic", "into_dynamic"),
            ("Id", "id"),
            ("asEntityRef", "as_entity_ref"),
            ("as_entity_ref_with", "as_entity_ref_with"),
            ("AsAnyEntityRef", "as_any_entity_ref"),
            ("asAnyEntityRefWith", "as_any_entity_ref_with"),
        ] {
            let entity = FsmEntityBuilder::new("Reserved".parse::<quent_schema::Path>().unwrap())
                .with_states([
                    state(state_name, &["done"], true, vec![]),
                    state("done", &[], false, vec![]),
                ])
                .build()
                .unwrap();

            assert!(matches!(
                entity_handle(&entity),
                Err(GenerateError::ReservedFsmHandleMethod {
                    state,
                    method,
                    ..
                }) if state == state_name && method == expected_method
            ));
        }
    }
}
