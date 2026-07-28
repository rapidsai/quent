// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of per-entity observers — the cheap-clone factories for handles.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::Entity;
use quote::quote;

use super::{event_ident, handle_ident, observer_ident};
use crate::common::to_case;

/// Generate the declaration of an {Entity}Observer and its impls.
pub(super) fn entity_observer(entity: &Entity) -> TokenStream {
    let entity_pascal = to_case(entity.path().name(), Case::Pascal);
    let event_ty = event_ident(entity);
    let observer_ty = observer_ident(entity);
    let handle_ty = handle_ident(entity);

    let observer_doc = format!(
        "Observer for `{entity_pascal}` entities. Obtain a per-instance handle \
         with [`Self::handle`]."
    );
    let handle_fn_doc = format!("Create a handle for a fresh `{entity_pascal}` instance.");
    let handle_with_id_doc =
        format!("Create a handle for the `{entity_pascal}` instance identified by `id`.");

    quote! {
        #[doc = #observer_doc]
        #[derive(Clone)]
        pub struct #observer_ty {
            inner: ::std::sync::Arc<::quent_instrumentation::Observer<#event_ty>>,
        }

        impl #observer_ty {
            #[doc = #handle_fn_doc]
            pub fn handle(&self) -> #handle_ty {
                #handle_ty {
                    inner: ::quent_instrumentation::Handle::new(
                        ::core::clone::Clone::clone(&self.inner),
                    ),
                }
            }

            #[doc = #handle_with_id_doc]
            pub fn handle_with_id(&self, id: ::quent_instrumentation::Uuid) -> #handle_ty {
                #handle_ty {
                    inner: ::quent_instrumentation::Handle::with_id(
                        id,
                        ::core::clone::Clone::clone(&self.inner),
                    ),
                }
            }
        }
    }
}
