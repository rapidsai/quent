// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of typed event-loader support.

use proc_macro2::TokenStream;
use quent_events_build::common::{relative_type_path, type_ident};
use quent_schema::Schema;
use quote::quote;

mod filesystem;

/// Generates model membership for entity markers and optional filesystem
/// loading.
pub(super) fn generate(schema: &Schema, filesystem: bool) -> TokenStream {
    let model = type_ident(schema.name());
    let stored_model = filesystem.then(|| filesystem::generate_model_impl(schema));
    let entities = schema.entities().map(|entity| {
        let marker = relative_type_path(entity.path(), &[], "");
        quote! {
            impl ::quent_store::event::EntityMarkerInModel<#model> for #marker {}
        }
    });
    quote! {
        #stored_model
        #(#entities)*
    }
}
