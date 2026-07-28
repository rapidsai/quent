// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of the schema context — builds every entity's observer on
//! construction and hands out cheap clones.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::Schema;
use quote::quote;

use super::{event_ident, observer_ident};
use crate::common::{raw_ident, to_case};

/// Generate the declaration of an {Schema}Context and its impls.
pub(super) fn schema_context(schema: &Schema) -> TokenStream {
    let schema_pascal = to_case(schema.name(), Case::Pascal);
    let context_ty = raw_ident(format!("{schema_pascal}Context"));
    let model_name = schema.name().to_string();

    let fields: Vec<_> = schema
        .entities()
        .map(|e| raw_ident(to_case(e.path().name(), Case::Snake)))
        .collect();
    let observer_tys: Vec<_> = schema.entities().map(observer_ident).collect();
    let event_tys: Vec<_> = schema.entities().map(event_ident).collect();
    let accessors: Vec<_> = schema
        .entities()
        .map(|e| {
            raw_ident(format!(
                "{}_observer",
                to_case(e.path().name(), Case::Snake)
            ))
        })
        .collect();
    let accessor_docs: Vec<String> = schema
        .entities()
        .map(|e| {
            format!(
                "Observer for `{}` entities.",
                to_case(e.path().name(), Case::Pascal)
            )
        })
        .collect();

    let context_doc = format!(
        "Instrumentation context for the `{model_name}` model. Construct it with \
         [`Self::try_new`], then call a `*_observer()` accessor to get an entity's \
         event observer, which creates the per-instance handles that emit events."
    );

    quote! {
        #[doc = #context_doc]
        pub struct #context_ty {
            #(#fields: #observer_tys,)*
            _inner: ::quent_instrumentation::Context,
        }

        impl #context_ty {
            /// Create a context, building every entity's exporter pipeline.
            /// Pass `None` for a no-op context that discards events.
            pub fn try_new(
                exporter: ::core::option::Option<::quent_instrumentation::ExporterOptions>,
            ) -> ::core::result::Result<Self, ::std::boxed::Box<dyn ::std::error::Error>> {
                Self::try_with_id(::quent_instrumentation::Uuid::now_v7(), exporter)
            }

            /// Create a context that adopts an existing `id` rather than
            /// generating one.
            pub fn try_with_id(
                id: ::quent_instrumentation::Uuid,
                exporter: ::core::option::Option<::quent_instrumentation::ExporterOptions>,
            ) -> ::core::result::Result<Self, ::std::boxed::Box<dyn ::std::error::Error>> {
                // With an exporter, build an active context, write the provenance
                // sidecar, then build each entity's observer using the exporter
                // options as its provider. `None` builds a no-op context and
                // no-op observers.
                let ( _inner, #(#fields,)* ) = match &exporter {
                    ::core::option::Option::Some(options) => {
                        let context = ::quent_instrumentation::Context::try_new(id)?;
                        ::quent_instrumentation::write_sidecar(options, id, Self::model_info());
                        let ( #(#fields,)* ) = context.block_on(async {
                            ::core::result::Result::<
                                _,
                                ::std::boxed::Box<dyn ::std::error::Error>,
                            >::Ok((
                                #(
                                    context
                                        .observer::<#event_tys>(::core::clone::Clone::clone(options))
                                        .await?,
                                )*
                            ))
                        })?;
                        ( context, #(#fields,)* )
                    }
                    ::core::option::Option::None => (
                        ::quent_instrumentation::Context::noop(id),
                        #( ::quent_instrumentation::Observer::<#event_tys>::noop(), )*
                    ),
                };
                ::core::result::Result::Ok(Self {
                    #( #fields: #observer_tys { inner: ::std::sync::Arc::new(#fields) }, )*
                    _inner,
                })
            }

            fn model_info() -> ::quent_instrumentation::build_info::ModelInfo {
                ::quent_instrumentation::build_info::ModelInfo {
                    name: #model_name.to_string(),
                    package: env!("CARGO_PKG_NAME").to_string(),
                    // No umbrella event enum on the schema-driven path; record
                    // the module the generated library is included into.
                    type_path: module_path!().to_string(),
                    source: ::quent_instrumentation::build_info::source_or_quent(
                        env!("CARGO_PKG_VERSION"),
                        option_env!("QUENT_SOURCE_REMOTE"),
                        option_env!("QUENT_SOURCE_COMMIT"),
                        option_env!("QUENT_SOURCE_BRANCH"),
                        option_env!("QUENT_SOURCE_DIRTY"),
                        option_env!("QUENT_SOURCE_BUILT_AT"),
                    ),
                    // The schema declares no analyzer entry.
                    analyzer_package: ::core::option::Option::None,
                }
            }

            /// Identity of this context.
            pub fn id(&self) -> ::quent_instrumentation::Uuid {
                self._inner.id()
            }

            #(
                #[doc = #accessor_docs]
                pub fn #accessors(&self) -> #observer_tys {
                    ::core::clone::Clone::clone(&self.#fields)
                }
            )*
        }
    }
}
