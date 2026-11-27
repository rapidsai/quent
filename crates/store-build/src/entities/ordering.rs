// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of payload ordering keys.

use proc_macro2::TokenStream;
use quent_events_build::common::{relative_type_path, type_ident};
use quent_schema::Schema;
use quote::quote;

use crate::GenerateError;

pub(crate) fn generate(schema: &Schema) -> Result<TokenStream, GenerateError> {
    let mut implementations = Vec::new();
    for entity in schema.entities() {
        let payload = relative_type_path(entity.path(), &[], "Event");
        let implementation = if quent_fsm::Fsm::try_from_entity(entity)?.is_some() {
            let sequence_field = quote::format_ident!("{}", quent_fsm::SEQUENCE_FIELD_NAME);
            let arms = entity.events().map(|event| {
                let variant = type_ident(event.name());
                quote! { Self::#variant { #sequence_field, .. } => *#sequence_field }
            });
            quote! {
                impl ::quent_store::fsm::Sequence for #payload {
                    fn sequence(&self) -> u16 {
                        match self { #(#arms,)* }
                    }
                }
                impl ::quent_store::OrderKey for #payload {
                    type Key = u16;

                    fn order_key(&self) -> u16 {
                        ::quent_store::fsm::Sequence::sequence(self)
                    }
                }
            }
        } else {
            quote! {
                impl ::quent_store::OrderKey for #payload {
                    type Key = ();

                    fn order_key(&self) {}
                }
            }
        };
        implementations.push(implementation);
    }
    Ok(quote! { #(#implementations)* })
}
