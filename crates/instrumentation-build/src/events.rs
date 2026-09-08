// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of per-entity event payload enums.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::Entity;
use quote::quote;

use crate::common::{derive_attr, doc_attr, doc_attr_or, path_name_pascal, raw_ident, to_case};
use crate::data_type::map_data_type;
use crate::{GenerateError, Options};

/// Re-export the runtime types used by event-only generated source.
pub(crate) fn reexports() -> TokenStream {
    quote! {
        pub use ::quent_events::{
            AnyEntity, DynamicAttributes, EntityRef, Event, Uuid,
        };
    }
}

/// Generate the schema entity marker and its event metadata implementation.
pub(crate) fn entity_types(entity: &Entity, opts: &Options) -> TokenStream {
    let marker = raw_ident(path_name_pascal(entity.path()));
    let event = raw_ident(format!("{}Event", path_name_pascal(entity.path())));
    let marker_doc = format!("Marker type for the `{}` entity.", entity.path());
    let stream_name = entity.path().to_string();
    let runtime = opts.event_runtime();
    let events_runtime = if opts.instrumentation {
        quote! { ::quent_instrumentation::events }
    } else {
        quote! { ::quent_events }
    };
    quote! {
        #[doc = #marker_doc]
        #[derive(Debug, Clone, Copy)]
        pub struct #marker;

        impl #runtime::EntityEvent for #event {
            const NAME: &'static str = #stream_name;
        }

        impl #events_runtime::Entity for #marker {
            type Event = #event;
        }
    }
}

pub(crate) fn entity_event_enum(
    entity: &Entity,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let entity_pascal = path_name_pascal(entity.path());
    let enum_ident = raw_ident(format!("{entity_pascal}Event"));
    let docs = doc_attr_or(
        entity.annotations().docs(),
        &format!("Events emitted by `{}` entities.", entity.path()),
    );
    let derives = derive_attr(opts.event_derives, opts.debug, opts.serde, opts.serde)?;
    let variants = entity
        .events()
        .map(|event| {
            let variant = raw_ident(to_case(event.name(), Case::Pascal));
            let variant_docs = doc_attr_or(
                event.annotations().docs(),
                &format!("The `{}` event.", event.name()),
            );
            let fields = event
                .fields()
                .map(|field| {
                    let name = raw_ident(to_case(field.name(), Case::Snake));
                    let ty = map_data_type(field.ty(), 0, entity.path().namespace(), opts)?;
                    let field_docs = doc_attr(field.annotations().docs());
                    Ok::<_, GenerateError>(quote! { #field_docs #name: #ty })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if fields.is_empty() {
                Ok(quote! { #variant_docs #variant })
            } else {
                Ok(quote! { #variant_docs #variant { #(#fields),* } })
            }
        })
        .collect::<Result<Vec<_>, GenerateError>>()?;
    Ok(quote! {
        #docs
        #derives
        pub enum #enum_ident {
            #(#variants),*
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pretty;
    use quent_schema::builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, SchemaBuilder};
    use quent_schema::test_utils::{entity, event, field, ident, record_type, schema};
    use quent_schema::{Annotations, Cardinality, DataType, Field};

    fn event_src(entity: &Entity) -> String {
        let opts = Options {
            debug: false,
            ..Options::default()
        };
        pretty(entity_event_enum(entity, &opts).unwrap())
    }

    #[test]
    fn data_type_mapping_covers_every_variant() {
        let s = schema(
            "M",
            [entity(
                "E",
                [event(
                    "ev",
                    [
                        field("b", DataType::Bool),
                        field("id", DataType::Uuid),
                        field("text", DataType::String),
                        field("n", DataType::U32),
                        field("opt", DataType::Option(Box::new(DataType::I32))),
                        field("list", DataType::List(Box::new(DataType::String))),
                        field("rec", record_type("SomeRecord")),
                        field("dynrec", DataType::DynamicRecord),
                        field(
                            "eref",
                            DataType::EntityRef {
                                data: None,
                                annotations: Annotations::default(),
                            },
                        ),
                        field(
                            "eref_payload",
                            DataType::EntityRef {
                                data: Some(Box::new(DataType::U64)),
                                annotations: Annotations::default(),
                            },
                        ),
                    ],
                )],
            )],
            [],
        );
        let expected = quote! {
            #[doc = "Events emitted by `E` entities."]
            pub enum EEvent {
                #[doc = "The `ev` event."]
                Ev {
                    b: bool,
                    id: ::quent_instrumentation::Uuid,
                    text: String,
                    n: u32,
                    opt: Option<i32>,
                    list: Vec<String>,
                    rec: SomeRecord,
                    dynrec: ::quent_instrumentation::DynamicAttributes,
                    eref: ::quent_instrumentation::EntityRef<AnyEntity>,
                    eref_payload: ::quent_instrumentation::EntityRef<AnyEntity, u64>
                }
            }
        };
        assert_eq!(event_src(s.entities().next().unwrap()), pretty(expected));
    }

    #[test]
    fn docs_annotations_become_doc_attributes() {
        let docs = |text: &str| AnnotationsBuilder::new().with_docs(text).build().unwrap();
        let field_x = Field::new(ident("x"), DataType::U8, docs("field doc"));
        let ev = EventBuilder::new(ident("ev"), Cardinality::Once)
            .with_field(field_x)
            .with_annotations(docs("event doc"))
            .build()
            .unwrap();
        let en = EntityBuilder::new(ident("E"))
            .with_event(ev)
            .with_annotations(docs("entity doc"))
            .build()
            .unwrap();
        let s = SchemaBuilder::new(ident("M"))
            .with_entity(en)
            .build()
            .unwrap();

        let expected = quote! {
            #[doc = "entity doc"]
            pub enum EEvent {
                #[doc = "event doc"]
                Ev {
                    #[doc = "field doc"]
                    x: u8
                }
            }
        };
        assert_eq!(event_src(s.entities().next().unwrap()), pretty(expected));
    }

    #[test]
    fn nested_container_types_recurse() {
        let s = schema(
            "M",
            [entity(
                "E",
                [event(
                    "ev",
                    [
                        field(
                            "nested",
                            DataType::Option(Box::new(DataType::List(Box::new(DataType::Option(
                                Box::new(DataType::U8),
                            ))))),
                        ),
                        field(
                            "eref_list",
                            DataType::EntityRef {
                                data: Some(Box::new(DataType::List(Box::new(DataType::String)))),
                                annotations: Annotations::default(),
                            },
                        ),
                    ],
                )],
            )],
            [],
        );
        let expected = quote! {
            #[doc = "Events emitted by `E` entities."]
            pub enum EEvent {
                #[doc = "The `ev` event."]
                Ev {
                    nested: Option<Vec<Option<u8>>>,
                    eref_list: ::quent_instrumentation::EntityRef<AnyEntity, Vec<String>>
                }
            }
        };
        assert_eq!(event_src(s.entities().next().unwrap()), pretty(expected));
    }

    #[test]
    fn keyword_and_digit_identifiers_are_handled() {
        let s = schema(
            "M",
            [entity(
                "Sig",
                // event named after a keyword -> Pascal "Type" needs no escape
                [event(
                    "type",
                    [
                        field("u8", DataType::U8),     // digit-safe: stays u8
                        field("type", DataType::U8),   // keyword field -> r#type
                        field("self", DataType::U8),   // un-rawable keyword -> self_
                        field("http2", DataType::U32), // digit-safe: stays http2
                    ],
                )],
            )],
            [],
        );
        let expected = quote! {
            #[doc = "Events emitted by `Sig` entities."]
            pub enum SigEvent {
                #[doc = "The `type` event."]
                Type {
                    u8: u8,
                    r#type: u8,
                    self_: u8,
                    http2: u32
                }
            }
        };
        assert_eq!(event_src(s.entities().next().unwrap()), pretty(expected));
    }
}
