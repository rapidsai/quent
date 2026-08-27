// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of the schema model used by the generic instrumentation context.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::{Entity, Schema};
use quote::quote;
use syn::Ident;

use super::model_ident;
use crate::GenerateError;
use crate::common::{module_ident, path_name_pascal, raw_ident, relative_type_path, to_case};
use crate::namespace::Namespace;

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
pub(super) fn schema_model(schema: &Schema, namespaces: &Namespace<'_>) -> TokenStream {
    let model = model_ident(schema);
    let observers = observers_ident(schema, namespaces);
    let observers_initializer = observer_storage_initializer(schema, namespaces);
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

    quote! {
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
                    ::core::result::Result::<
                        _,
                        ::std::boxed::Box<dyn ::std::error::Error>,
                    >::Ok(#observers_initializer)
                })
            }
        }
    }
}

fn observer_storage_initializer(schema: &Schema, namespace: &Namespace<'_>) -> TokenStream {
    let storage = observers_path(schema, namespace);
    let entity_fields = namespace.entities().iter().map(|entity| {
        let field = entity_observer_field(entity);
        let entity_ty = relative_type_path(entity.path(), &[], "");
        let event_ty = relative_type_path(entity.path(), &[], "Event");
        let observer = quote! {
            context
                .observer::<#event_ty>(provider)
                .await?
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
        let value = observer_storage_initializer(schema, child);
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
