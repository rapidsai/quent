// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of filesystem event-loader import descriptors.

use proc_macro2::TokenStream;
use quent_events_build::common::{relative_type_path, type_ident};
use quent_schema::Schema;
use quote::quote;

/// Generates code that lets the model load events from the filesystem.
///
/// It lists one event importer for each entity, in schema order.
pub(super) fn generate_model_impl(schema: &Schema) -> TokenStream {
    let model = type_ident(schema.name());
    let importers = schema.entities().map(|entity| {
        let marker = relative_type_path(entity.path(), &[], "");
        quote! {
            ::quent_store::event::filesystem::EventImporter::<Self>::import_for_entity::<#marker>()
        }
    });
    quote! {
        impl ::quent_store::event::filesystem::Model for #model {
            const EVENT_IMPORTERS:
                &'static [::quent_store::event::filesystem::EventImporter<Self>] = &[
                    #(#importers,)*
                ];
        }
    }
}
