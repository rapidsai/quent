// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of the live instrumentation surface: per-entity observers and
//! handles, plus the schema's context that deals them out.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::{Entity, Schema};
use quote::quote;
use syn::Ident;

use crate::GenerateError;
use crate::common::{raw_ident, to_case};

mod context;
mod handle;
mod observer;

pub(crate) use handle::MAX_ONCE_EVENTS;

/// The full instrumentation surface for `schema`: per entity, an `EntityEvent`
/// impl, an observer, and a handle; then the `{Schema}Context` that builds and
/// hands out the observers.
///
/// # Errors
///
/// Returns [`GenerateError::TooManyOnceEvents`] if an entity declares more
/// once-cardinality events than the per-handle flag word holds.
pub(crate) fn generate_runtime_types(schema: &Schema) -> Result<TokenStream, GenerateError> {
    let entities: Vec<TokenStream> = schema
        .entities()
        .map(|entity| {
            let marker = entity_marker(entity);
            let event_impl = entity_event_impl(entity);
            let observer = observer::entity_observer(entity);
            let handle = handle::entity_handle(entity)?;
            Ok::<_, GenerateError>(quote! {
                #marker
                #event_impl
                #observer
                #handle
            })
        })
        .collect::<Result<_, _>>()?;
    let context = context::schema_context(schema);
    Ok(quote! {
        #(#entities)*
        #context
    })
}

/// Re-export the always-available runtime types that appear in the generated
/// API, so consumers reference them through the generated module rather than
/// `quent_instrumentation`. Opt-in types like the callback exporter are
/// not re-exported.
pub(crate) fn reexports() -> TokenStream {
    quote! {
        pub use ::quent_instrumentation::{
            AnyEntity, DynamicAttributes, EntityRef, Event, HandleError, Uuid,
        };
    }
}

/// `{Entity}` — the zero-size marker naming the entity, used as the target
/// type of [`EntityRef`](quent_instrumentation::EntityRef) fields that point at it.
fn entity_marker(entity: &Entity) -> TokenStream {
    let marker = marker_ident(entity);
    let doc = format!(
        "Marker type for the `{}` entity.",
        to_case(entity.path().name(), Case::Pascal)
    );
    quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, Copy)]
        pub struct #marker;
    }
}

/// Tie an entity's event enum to its stream name (the entity's snake-case name).
fn entity_event_impl(entity: &Entity) -> TokenStream {
    let event_ty = event_ident(entity);
    let stream_name = to_case(entity.path().name(), Case::Snake);
    quote! {
        impl ::quent_instrumentation::EntityEvent for #event_ty {
            const NAME: &'static str = #stream_name;
        }
    }
}

/// `{Entity}Event` — the entity's event enum.
fn event_ident(entity: &Entity) -> Ident {
    raw_ident(format!(
        "{}Event",
        to_case(entity.path().name(), Case::Pascal)
    ))
}

/// `{Entity}` — the entity's ref-target marker type.
fn marker_ident(entity: &Entity) -> Ident {
    raw_ident(to_case(entity.path().name(), Case::Pascal))
}

/// `{Entity}Observer`.
fn observer_ident(entity: &Entity) -> Ident {
    raw_ident(format!(
        "{}Observer",
        to_case(entity.path().name(), Case::Pascal)
    ))
}

/// `{Entity}Handle`.
fn handle_ident(entity: &Entity) -> Ident {
    raw_ident(format!(
        "{}Handle",
        to_case(entity.path().name(), Case::Pascal)
    ))
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
        let src = pretty(generate_runtime_types(&s).unwrap());
        assert!(src.contains("impl ::quent_instrumentation::EntityEvent for ConnectionEvent"));
        assert!(src.contains(r#"const NAME: &'static str = "connection""#));
        assert!(src.contains("pub struct ConnectionObserver"));
        assert!(src.contains("pub struct ConnectionHandle"));
        assert!(src.contains("pub struct DemoContext"));
    }
}
