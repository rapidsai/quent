// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of the live instrumentation surface: per-entity observers and
//! handles, plus the schema's context that deals them out.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::{Entity, Schema};
use quote::quote;
use syn::Ident;

use crate::GenerateError;
use crate::Options;
use crate::common::{path_name_pascal, raw_ident, relative_root_type, to_case};

mod context;
mod handle;

pub(crate) use handle::MAX_ONCE_EVENTS;

pub(crate) fn entity_runtime_types(
    schema: &Schema,
    entity: &Entity,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let handle = handle::entity_handle(entity, opts)?;
    let entity_impl = entity_impl(schema, entity);
    Ok(quote! {
        #handle
        #entity_impl
    })
}

pub(crate) fn generate_model(
    schema: &Schema,
    namespaces: &crate::namespace::Namespace<'_>,
) -> TokenStream {
    context::schema_model(schema, namespaces)
}

pub(crate) fn observer_storage(
    schema: &Schema,
    namespace: &crate::namespace::Namespace<'_>,
) -> Result<TokenStream, GenerateError> {
    context::observer_storage(schema, namespace)
}

pub(crate) fn entity_types(schema: &Schema) -> TokenStream {
    let model = model_ident(schema);
    let model_name = schema.name().to_string();
    let handle_docs =
        format!("Handle to one entity instance in the `{model_name}` instrumentation model.");
    quote! {
        #[doc = #handle_docs]
        pub struct Handle<E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>> {
            inner: ::quent_instrumentation::HandleInner<E>,
        }

        impl<E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>>
            ::core::convert::From<::quent_instrumentation::HandleInner<E>> for Handle<E>
        {
            fn from(inner: ::quent_instrumentation::HandleInner<E>) -> Self {
                Self { inner }
            }
        }

        impl<E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>> ::core::ops::Deref
            for Handle<E>
        {
            type Target = ::quent_instrumentation::HandleInner<E>;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }
    }
}

/// Re-export the always-available runtime types that appear in the generated
/// API, so consumers reference them through the generated module rather than
/// `quent_instrumentation`. Opt-in types like the callback exporter are
/// not re-exported.
pub(crate) fn reexports() -> TokenStream {
    quote! {
        pub use ::quent_instrumentation::{
            AnyEntity, Context, DynamicAttributes, EntityRef, Event, HandleError, Observer, Uuid,
        };
    }
}

/// `{Entity}Event` — the entity's event enum.
fn event_ident(entity: &Entity) -> Ident {
    raw_ident(format!("{}Event", path_name_pascal(entity.path())))
}

/// `{Entity}` — the entity's ref-target marker type.
fn marker_ident(entity: &Entity) -> Ident {
    raw_ident(path_name_pascal(entity.path()))
}

pub(super) fn model_ident(schema: &Schema) -> Ident {
    raw_ident(to_case(schema.name(), Case::Pascal))
}

fn entity_impl(schema: &Schema, entity: &Entity) -> TokenStream {
    let namespace = entity.path().namespace();
    let marker = marker_ident(entity);
    let context = relative_root_type("Context", namespace);
    let model_name = model_ident(schema).to_string();
    let model = relative_root_type(&model_name, namespace);
    let handle = relative_root_type("Handle", namespace);
    quote! {
        impl ::quent_instrumentation::InstrumentedEntity for #marker {
            type Context = #context<#model>;
            type Handle = #handle<Self>;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pretty;
    use quent_schema::Cardinality;
    use quent_schema::DataType;
    use quent_schema::builder::{EntityBuilder, EventBuilder, SchemaBuilder};
    use quent_schema::test_utils::{field, ident};

    #[test]
    fn generate_assembles_event_impl_observer_handle_and_context() {
        let connection = EntityBuilder::new(ident("Connection"))
            .with_event(
                EventBuilder::new(ident("data"), Cardinality::Multi)
                    .with_field(field("bytes", DataType::U64))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let s = SchemaBuilder::new(ident("Demo"))
            .with_entity(connection)
            .build()
            .unwrap();
        let entity = s.entities().next().unwrap();
        let event_types = crate::events::entity_types(entity, &Options::default());
        let entity_types = entity_runtime_types(&s, entity, &Options::default()).unwrap();
        let namespaces = crate::namespace::Namespace::root(&s);
        let generated_model = crate::model::generate(&s, &namespaces, &Options::default()).unwrap();
        let model = generate_model(&s, &namespaces);
        let src = pretty(quote! {
            #event_types
            #entity_types
            #generated_model
            #model
        });
        assert!(src.contains("type Event = ConnectionEvent"));
        assert!(src.contains("impl Handle<Connection>"));
        assert!(src.contains("pub struct Demo"));
    }
}
