// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of typed event-loader support.

use proc_macro2::TokenStream;
use quent_schema::Schema;
use quote::quote;

mod filesystem;

pub(super) fn generate(schema: &Schema, filesystem: bool) -> TokenStream {
    let model = quent_instrumentation_build::generated_model_path(schema);
    let stored_model = filesystem.then(|| filesystem::generate(schema));
    let entities = schema.entities().map(|entity| {
        let marker = quent_instrumentation_build::generated_entity_path(entity);
        quote! {
            impl ::quent_store::event::EntityMarkerInModel<#model> for #marker {}
        }
    });
    quote! {
        #stored_model
        #(#entities)*
    }
}
