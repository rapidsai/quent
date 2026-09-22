// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of the schema model used by the generic instrumentation context.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::{Entity, Schema};
use quote::quote;
use syn::Ident;

use super::model_ident;
use crate::common::{module_ident, path_name_pascal, raw_ident, relative_type_path, to_case};
use crate::namespace::Namespace;
use crate::nvtx::CaptureConfig;
use crate::{GenerateError, Options};

/// Generate observer storage for one schema namespace.
pub(super) fn observer_storage(
    schema: &Schema,
    namespace: &Namespace<'_>,
) -> Result<TokenStream, GenerateError> {
    if !namespace.path().is_empty() && !namespace.has_entities() {
        return Ok(quote! {});
    }

    let storage = observers_ident(schema, namespace);
    let storage_name = storage.to_string();
    if let Some(schema_path) = namespace
        .records()
        .iter()
        .map(|record| record.path())
        .chain(namespace.entities().iter().map(|entity| entity.path()))
        .find(|path| path_name_pascal(path) == storage_name)
    {
        return Err(GenerateError::GeneratedTypeCollision {
            generated: storage_name,
            schema_path: schema_path.clone(),
        });
    }

    let (visibility, field_visibility) = storage_visibility(namespace);
    let description = if namespace.path().is_empty() {
        format!(
            "Observers for the `{}` instrumentation model.",
            schema.name()
        )
    } else {
        format!(
            "Observers for the `{}` namespace.",
            namespace
                .path()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("::")
        )
    };
    let hidden_docs = "Hidden because the model context provides typed observer access.";
    let entity_fields = namespace.entities().iter().map(|entity| {
        let field = entity_observer_field(entity);
        let entity_ty = relative_type_path(entity.path(), namespace.path(), "");
        quote! {
            #field_visibility #field: ::quent_instrumentation::Observer<#entity_ty>
        }
    });
    let namespace_fields = namespace.children_with_entities().map(|child| {
        let segment = child
            .path()
            .last()
            .expect("child namespaces extend their parent");
        let field = namespace_observers_field(segment);
        let module = module_ident(segment);
        let child_storage = observers_ident(schema, child);
        quote! {
            #field_visibility #field: #module::#child_storage
        }
    });

    Ok(quote! {
        #[doc = #description]
        #[doc = ""]
        #[doc = #hidden_docs]
        #[doc(hidden)]
        #visibility struct #storage {
            #(#entity_fields,)*
            #(#namespace_fields,)*
        }
    })
}

/// Generate the model's observer integration.
pub(super) fn schema_model(
    schema: &Schema,
    namespaces: &Namespace<'_>,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let model = model_ident(schema);
    let observers = observers_ident(schema, namespaces);
    let capture = crate::nvtx::capture_config(schema, opts)?;
    let capture_prelude = capture.as_ref().map(nvtx_capture_prelude);
    let observers_initializer = observer_storage_initializer(schema, namespaces, capture.as_ref());
    let provider_binding = if schema.entities().next().is_some() {
        raw_ident("provider".to_owned())
    } else {
        raw_ident("_provider".to_owned())
    };
    let provider_event_types = schema
        .entities()
        .map(|entity| relative_type_path(entity.path(), &[], "Event"))
        .collect::<Vec<_>>();
    let provider_bounds = (!provider_event_types.is_empty()).then(|| {
        quote! {
            where
                #(P: ::quent_instrumentation::ExporterProvider<#provider_event_types>,)*
        }
    });
    let observer_impls = schema
        .entities()
        .map(|entity| observer_storage_impl(schema, entity));
    let collector_sink = opts.collector_sink.then(|| collector_sink_impl(schema));

    Ok(quote! {
        #(#observer_impls)*

        impl ::quent_instrumentation::InstrumentedModel for #model {
            type Observers = #observers;
        }

        impl<P> ::quent_instrumentation::ObserverBuilder<P> for #model
        #provider_bounds
        {
            fn build_observers(
                context: &::quent_instrumentation::ContextInner,
                #provider_binding: &P,
            ) -> ::core::result::Result<
                Self::Observers,
                ::std::boxed::Box<dyn ::std::error::Error>,
            > {
                context.block_on(async {
                    #capture_prelude
                    ::core::result::Result::<
                        _,
                        ::std::boxed::Box<dyn ::std::error::Error>,
                    >::Ok(#observers_initializer)
                })
            }
        }

        #collector_sink
    })
}

fn nvtx_capture_prelude(_capture: &CaptureConfig) -> TokenStream {
    let event_ty = relative_type_path(&nvtx_schema::nvtx_event_path(), &[], "Event");
    quote! {
        let __quent_nvtx_inner = context
            .observer::<#event_ty>(provider)
            .await?;
        let __quent_nvtx_sender = __quent_nvtx_inner.sender();
        let __quent_nvtx_context_id = context.id();
    }
}

fn collector_sink_impl(schema: &Schema) -> TokenStream {
    let model = model_ident(schema);
    let routes = schema.entities().map(|entity| {
        let marker = relative_type_path(entity.path(), &[], "");
        let event_ty = relative_type_path(entity.path(), &[], "Event");
        quote! {
            if entity == <#event_ty as ::quent_instrumentation::EntityEvent>::NAME {
                let event = ::quent_instrumentation::deserialize_event::<#event_ty>(event)?;
                context.observer::<#marker>().forward(event);
                return ::core::result::Result::Ok(());
            }
        }
    });

    quote! {
        #[cfg(feature = "collector")]
        impl ::quent_instrumentation::CollectorRouter for #model {
            fn dispatch(
                context: &Context<Self>,
                entity: &str,
                event: &[u8],
            ) -> ::core::result::Result<
                (),
                ::std::boxed::Box<dyn ::std::error::Error>,
            > {
                #(#routes)*
                ::core::result::Result::Err(
                    ::std::format!("unknown entity stream `{entity}`").into(),
                )
            }
        }
    }
}

fn observer_storage_initializer(
    schema: &Schema,
    namespace: &Namespace<'_>,
    capture: Option<&CaptureConfig>,
) -> TokenStream {
    let storage = observers_path(schema, namespace);
    let entity_fields = namespace.entities().iter().map(|entity| {
        let field = entity_observer_field(entity);
        let entity_ty = relative_type_path(entity.path(), &[], "");
        let event_ty = relative_type_path(entity.path(), &[], "Event");
        let observer = if capture.is_some() && entity.path() == &nvtx_schema::nvtx_event_path() {
            quote! { __quent_nvtx_inner }
        } else {
            quote! {
                context
                    .observer::<#event_ty>(provider)
                    .await?
            }
        };
        let observer = if capture.is_some_and(|capture| &capture.process == entity.path()) {
            let nvtx_event_ty = relative_type_path(&nvtx_schema::nvtx_event_path(), &[], "Event");
            quote! {
                {
                    let __quent_nvtx_process_sender =
                        ::core::clone::Clone::clone(&__quent_nvtx_sender);
                    (#observer).with_emit_activation(move |__quent_nvtx_process_id| {
                        let __quent_nvtx_stream_id = ::quent_instrumentation::Uuid::now_v7();
                        let __quent_nvtx_binding = ::quent_instrumentation::Event::new_now(
                            __quent_nvtx_stream_id,
                            #nvtx_event_ty::Initialized {
                                process: ::quent_instrumentation::EntityRef::new(
                                    __quent_nvtx_process_id,
                                    (),
                                ),
                            },
                        );
                        let __quent_nvtx_gate = ::std::sync::Arc::new(
                            ::std::sync::Mutex::new(::core::option::Option::Some(
                                ::std::vec::Vec::<
                                    ::quent_instrumentation::Event<#nvtx_event_ty>
                                >::new(),
                            )),
                        );
                        let __quent_nvtx_hook_gate =
                            ::std::sync::Arc::clone(&__quent_nvtx_gate);
                        let __quent_nvtx_hook_sender =
                            ::core::clone::Clone::clone(&__quent_nvtx_process_sender);
                        ::nvtx_injection::register_source(
                            ::nvtx_injection::SourceBinding {
                                context_id: __quent_nvtx_context_id,
                                process_id: __quent_nvtx_process_id,
                                stream_id: __quent_nvtx_stream_id,
                            },
                            move |__quent_nvtx_event| {
                                let __quent_nvtx_event = ::quent_instrumentation::Event::new_now(
                                    __quent_nvtx_stream_id,
                                    ::core::convert::Into::<#nvtx_event_ty>::into(
                                        __quent_nvtx_event,
                                    ),
                                );
                                let mut __quent_nvtx_gate = __quent_nvtx_hook_gate
                                    .lock()
                                    .unwrap_or_else(::std::sync::PoisonError::into_inner);
                                match __quent_nvtx_gate.as_mut() {
                                    ::core::option::Option::Some(__quent_nvtx_buffer) => {
                                        __quent_nvtx_buffer.push(__quent_nvtx_event);
                                    }
                                    ::core::option::Option::None => {
                                        __quent_nvtx_hook_sender.send(__quent_nvtx_event);
                                    }
                                }
                            },
                        )
                        .map_err(::quent_instrumentation::HandleError::source_activation)?;
                        __quent_nvtx_process_sender.send(__quent_nvtx_binding);
                        let mut __quent_nvtx_gate = __quent_nvtx_gate
                            .lock()
                            .unwrap_or_else(::std::sync::PoisonError::into_inner);
                        for __quent_nvtx_event in __quent_nvtx_gate
                            .take()
                            .expect("NVTX activation gate opens only once")
                        {
                            __quent_nvtx_process_sender.send(__quent_nvtx_event);
                        }
                        ::core::result::Result::Ok(())
                    })
                }
            }
        } else {
            observer
        };
        quote! {
            #field: ::quent_instrumentation::Observer::<#entity_ty>::new(
                ::std::sync::Arc::new(#observer),
            )
        }
    });
    let namespace_fields = namespace.children_with_entities().map(|child| {
        let segment = child
            .path()
            .last()
            .expect("child namespaces extend their parent");
        let field = namespace_observers_field(segment);
        let value = observer_storage_initializer(schema, child, capture);
        quote! { #field: #value }
    });
    quote! {
        #storage {
            #(#entity_fields,)*
            #(#namespace_fields,)*
        }
    }
}

fn observer_storage_impl(schema: &Schema, entity: &Entity) -> TokenStream {
    let storage = root_observers_ident(schema);
    let entity_ty = relative_type_path(entity.path(), &[], "");
    let mut observer = quote! { self };
    for segment in entity.path().namespace() {
        let field = namespace_observers_field(segment);
        observer = quote! { #observer.#field };
    }
    let field = entity_observer_field(entity);
    observer = quote! { #observer.#field };

    quote! {
        impl ::quent_instrumentation::ObserverProvider<#entity_ty> for #storage {
            fn observer(&self) -> ::quent_instrumentation::Observer<#entity_ty> {
                ::core::clone::Clone::clone(&#observer)
            }
        }
    }
}

fn observers_ident(schema: &Schema, namespace: &Namespace<'_>) -> Ident {
    match namespace.path().last() {
        Some(segment) => raw_ident(format!("{}Observers", to_case(segment, Case::Pascal))),
        None => root_observers_ident(schema),
    }
}

fn root_observers_ident(schema: &Schema) -> Ident {
    raw_ident(format!("{}Observers", to_case(schema.name(), Case::Pascal)))
}

fn observers_path(schema: &Schema, namespace: &Namespace<'_>) -> TokenStream {
    let modules = namespace.path().iter().map(module_ident);
    let storage = observers_ident(schema, namespace);
    quote! { #(#modules::)* #storage }
}

fn entity_observer_field(entity: &Entity) -> Ident {
    raw_ident(format!(
        "{}_observer",
        to_case(entity.path().name(), Case::Snake)
    ))
}

fn namespace_observers_field(segment: &quent_schema::Identifier) -> Ident {
    raw_ident(format!("{}_observers", to_case(segment, Case::Snake)))
}

fn storage_visibility(namespace: &Namespace<'_>) -> (TokenStream, TokenStream) {
    if namespace.path().is_empty() {
        return (quote! { pub }, quote! {});
    }
    let parents = namespace.path().iter().map(|_| quote! { super });
    let root = quote! { #(#parents)::* };
    let visibility = quote! { pub(in #root) };
    (visibility.clone(), visibility)
}
