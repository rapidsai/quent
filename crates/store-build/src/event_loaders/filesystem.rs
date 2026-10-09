// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of filesystem event-loader import descriptors.

use proc_macro2::TokenStream;
use quent_schema::Schema;
use quote::quote;

pub(super) fn generate(schema: &Schema) -> TokenStream {
    let model = quent_instrumentation_build::generated_model_path(schema);
    let importers = schema.entities().map(|entity| {
        let marker = quent_instrumentation_build::generated_entity_path(entity);
        quote! {
            ::quent_store::event::filesystem::EventImporter::<#model>::import_for_entity::<#marker>()
        }
    });
    quote! {
        impl ::quent_store::event::filesystem::Model for #model {
            fn event_importers(
            ) -> &'static [::quent_store::event::filesystem::EventImporter<Self>] {
                static IMPORTERS: &[
                    ::quent_store::event::filesystem::EventImporter<#model>
                ] = &[
                    #(#importers,)*
                ];
                IMPORTERS
            }
        }
    }
}
