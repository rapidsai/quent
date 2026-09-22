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
    let capture = crate::nvtx::capture_config(schema, opts)?;
    let activation = capture
        .as_ref()
        .filter(|capture| &capture.process == entity.path());
    let handle = handle::entity_handle(entity, opts, activation)?;
    let entity_impl = entity_impl(schema, entity, &handle.associated_type);
    let handle = handle.tokens;
    Ok(quote! {
        #handle
        #entity_impl
    })
}

pub(crate) fn generate_model(
    schema: &Schema,
    namespaces: &crate::namespace::Namespace<'_>,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    context::schema_model(schema, namespaces, opts)
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
    let fsm_handle = handle::fsm_handle_type(schema);
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

        impl<E: ::quent_instrumentation::InstrumentedEntity<Context = Context<#model>>> Handle<E>
        {
            /// Returns the entity instance ID.
            pub fn uuid(&self) -> ::quent_instrumentation::Uuid {
                self.inner.uuid()
            }

            /// Returns a typed reference to this instance carrying no data.
            pub fn as_entity_ref(&self) -> ::quent_instrumentation::EntityRef<E> {
                self.inner.as_entity_ref()
            }

            /// Returns a typed reference to this instance carrying `data`.
            pub fn as_entity_ref_with<T>(&self, data: T) -> ::quent_instrumentation::EntityRef<E, T> {
                self.inner.as_entity_ref_with(data)
            }

            /// Returns an untyped reference to this instance carrying no data.
            pub fn as_any_entity_ref(&self) -> ::quent_instrumentation::EntityRef<::quent_instrumentation::AnyEntity> {
                self.inner.as_any_entity_ref()
            }

            /// Returns an untyped reference to this instance carrying `data`.
            pub fn as_any_entity_ref_with<T>(
                &self,
                data: T,
            ) -> ::quent_instrumentation::EntityRef<::quent_instrumentation::AnyEntity, T> {
                self.inner.as_any_entity_ref_with(data)
            }
        }

        #fsm_handle
    }
}

/// Re-export the always-available runtime types that appear in the generated
/// API, so consumers reference them through the generated module rather than
/// `quent_instrumentation`. Opt-in types like the callback exporter are
/// not re-exported.
pub(crate) fn reexports() -> TokenStream {
    quote! {
        pub use ::quent_instrumentation::{
            AnyEntity, Context, ContextOptions, DynamicAttribute, DynamicAttributes, DynamicList,
            DynamicStruct, DynamicNull, DynamicValue, EntityRef, Event, HandleError, Noop,
            Observer, SourceCapture, Uuid,
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

fn entity_impl(schema: &Schema, entity: &Entity, handle_type: &TokenStream) -> TokenStream {
    let namespace = entity.path().namespace();
    let marker = marker_ident(entity);
    let context = relative_root_type("Context", namespace);
    let model_name = model_ident(schema).to_string();
    let model = relative_root_type(&model_name, namespace);
    quote! {
        impl ::quent_instrumentation::InstrumentedEntity for #marker {
            type Context = #context<#model>;
            type Handle = #handle_type;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pretty;
    use quent_fsm::{FsmEntityBuilder, StateDecl};
    use quent_schema::Cardinality;
    use quent_schema::DataType;
    use quent_schema::builder::{EntityBuilder, EventBuilder, SchemaBuilder};
    use quent_schema::test_utils::{entity, event, field, ident};

    fn fsm_state(name: &str, to: &[&str], initial: bool) -> StateDecl {
        StateDecl {
            name: ident(name),
            attributes: vec![],
            to: to.iter().map(|target| ident(target)).collect(),
            initial,
        }
    }

    #[test]
    fn reexports_dynamic_attribute_value_types() {
        let source = pretty(reexports());

        for name in [
            "DynamicAttribute",
            "DynamicAttributes",
            "DynamicList",
            "DynamicStruct",
            "DynamicValue",
            "DynamicNull",
        ] {
            assert!(source.contains(name), "missing re-export for {name}");
        }
    }

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
        let model = generate_model(&s, &namespaces, &Options::default()).unwrap();
        let src = pretty(quote! {
            #event_types
            #entity_types
            #generated_model
            #model
        });
        assert!(src.contains("type Event = ConnectionEvent"));
        assert!(src.contains("impl Handle<Connection>"));
        assert!(src.contains("pub struct Demo"));
        assert!(src.contains("impl<P> ::quent_instrumentation::ObserverBuilder<P> for Demo"));
        assert!(src.contains("P: ::quent_instrumentation::ExporterProvider<ConnectionEvent>"));
    }

    #[test]
    fn fsm_observers_deal_out_fsm_handles() {
        let query = FsmEntityBuilder::new("Query".parse::<quent_schema::Path>().unwrap())
            .with_states([
                fsm_state("submitted", &["ready"], true),
                fsm_state("ready", &[], false),
            ])
            .build()
            .unwrap();
        let schema = SchemaBuilder::new(ident("Demo"))
            .with_entity(query)
            .build()
            .unwrap();

        let source = crate::generate_str(&schema, &Options::default()).unwrap();

        assert!(source.contains("pub struct FsmHandle<"));
        assert!(source.contains("type Handle = FsmHandle<Self>"));
        assert!(source.contains("impl ::quent_instrumentation::FsmEvent for QueryEvent"));
    }

    #[test]
    fn generates_provider_observers_for_nested_namespaces() {
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity("Foo::Query", [event("created", [])]))
            .with_entity(entity("Foo::Nested::Task", [event("created", [])]))
            .build()
            .unwrap();
        let namespaces = crate::namespace::Namespace::root(&schema);

        let src = pretty(generate_model(&schema, &namespaces, &Options::default()).unwrap());

        assert!(src.contains("P: ::quent_instrumentation::ExporterProvider<foo::QueryEvent>"));
        assert!(
            src.contains("P: ::quent_instrumentation::ExporterProvider<foo::nested::TaskEvent>")
        );
        assert!(src.contains("context.observer::<foo::QueryEvent>(provider)"));
        assert!(src.contains("context.observer::<foo::nested::TaskEvent>(provider)"));
        assert!(src.contains("foo::FooObservers"));
        assert!(src.contains("foo::nested::NestedObservers"));
    }
}
