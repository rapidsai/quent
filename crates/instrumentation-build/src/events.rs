// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of per-entity event payload enums.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::{Entity, Schema};
use quote::quote;

use crate::common::{derive_attr, doc_attr, doc_attr_or, raw_ident, to_case};
use crate::data_type::map_data_type;
use crate::{GenerateError, Options};

pub(crate) fn generate_event_types(
    schema: &Schema,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let enums: Vec<TokenStream> = schema
        .entities()
        .map(|entity| entity_event_enum(entity, opts))
        .collect::<Result<_, _>>()?;
    Ok(quote! { #(#enums)* })
}

fn entity_event_enum(entity: &Entity, opts: &Options) -> Result<TokenStream, GenerateError> {
    let entity_pascal = to_case(entity.name(), Case::Pascal);
    let enum_ident = raw_ident(format!("{entity_pascal}Event"));
    let docs = doc_attr_or(
        entity.annotations().docs(),
        &format!("Events emitted by `{entity_pascal}` entities."),
    );
    let derives = derive_attr(opts.event_derives)?;
    let variants: Vec<TokenStream> = entity
        .events()
        .map(|event| {
            let variant = raw_ident(to_case(event.name(), Case::Pascal));
            let variant_docs = doc_attr_or(
                event.annotations().docs(),
                &format!("The `{}` event.", event.name()),
            );
            let fields: Vec<TokenStream> = event
                .fields()
                .map(|field| {
                    let name = raw_ident(to_case(field.name(), Case::Snake));
                    let ty = map_data_type(field.ty(), 0);
                    let field_docs = doc_attr(field.annotations().docs());
                    quote! { #field_docs #name: #ty }
                })
                .collect();
            if fields.is_empty() {
                quote! { #variant_docs #variant }
            } else {
                quote! { #variant_docs #variant { #(#fields),* } }
            }
        })
        .collect();
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
    use quent_schema::test_utils::{entity, event, field, ident, schema};
    use quent_schema::{Annotations, Cardinality, DataType, Field};

    fn events_src(s: &Schema) -> String {
        pretty(generate_event_types(s, &Options::default()).unwrap())
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
                        field("rec", DataType::Record(ident("SomeRecord"))),
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
        assert_eq!(events_src(&s), pretty(expected));
    }

    #[test]
    fn docs_annotations_become_doc_attributes() {
        let docs = |text: &str| {
            let mut builder = AnnotationsBuilder::new();
            builder.set_docs(text);
            builder.build()
        };
        let field_x = Field::new(ident("x"), DataType::U8, docs("field doc"));
        let ev = EventBuilder::new(ident("ev"), Cardinality::Once)
            .try_with_field(field_x)
            .unwrap()
            .with_annotations(docs("event doc"))
            .build();
        let en = EntityBuilder::new(ident("E"))
            .try_with_event(ev)
            .unwrap()
            .with_annotations(docs("entity doc"))
            .build();
        let s = SchemaBuilder::new(ident("M"))
            .try_with_entity(en)
            .unwrap()
            .build();

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
        assert_eq!(events_src(&s), pretty(expected));
    }

    #[test]
    fn multiple_entities_emit_in_declaration_order() {
        let s = schema(
            "M",
            [
                entity("Alpha", [event("started", [field("id", DataType::U32)])]),
                entity("Beta", [event("ended", [])]),
            ],
            [],
        );
        let expected = quote! {
            #[doc = "Events emitted by `Alpha` entities."]
            pub enum AlphaEvent {
                #[doc = "The `started` event."]
                Started { id: u32 }
            }
            #[doc = "Events emitted by `Beta` entities."]
            pub enum BetaEvent {
                #[doc = "The `ended` event."]
                Ended
            }
        };
        assert_eq!(events_src(&s), pretty(expected));
    }

    #[test]
    fn entity_without_events_emits_empty_enum() {
        let s = schema("M", [entity("E", [])], []);
        let expected = quote! {
            #[doc = "Events emitted by `E` entities."]
            pub enum EEvent {}
        };
        assert_eq!(events_src(&s), pretty(expected));
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
        assert_eq!(events_src(&s), pretty(expected));
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
        assert_eq!(events_src(&s), pretty(expected));
    }
}
