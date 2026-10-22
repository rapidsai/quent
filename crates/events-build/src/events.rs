// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of per-entity event payload enums.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::Entity;
use quote::quote;

use crate::{
    Error, Options, Runtime,
    common::{derive_attr, doc_attr, doc_attr_or, path_name_pascal, raw_ident, to_case},
    data_type::map_data_type,
};

/// Generates `pub use` statements that make shared types such as `Event` and
/// `EntityRef` available through the generated module.
pub(crate) fn reexports(event_runtime: &Runtime) -> TokenStream {
    let runtime = &event_runtime.events;
    quote! {
        pub use #runtime::{
            AnyEntity, DynamicAttributes, EntityRef, Event, Uuid,
        };
    }
}

/// Generates a Rust marker type for a schema entity and links it to that entity's
/// event payload enum. Also gives the event payload enum its schema entity name.
pub(crate) fn entity_types(entity: &Entity, event_runtime: &Runtime) -> TokenStream {
    let marker = raw_ident(path_name_pascal(entity.path()));
    let event = raw_ident(format!("{}Event", path_name_pascal(entity.path())));
    let marker_doc = format!("Marker type for the `{}` entity.", entity.path());
    let stream_name = entity.path().to_string();
    let runtime = &event_runtime.events;
    let entity_marker = &event_runtime.entity_marker;
    quote! {
        #[doc = #marker_doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct #marker;

        impl #runtime::EventPayload for #event {
            const NAME: &'static str = #stream_name;
        }

        impl #entity_marker for #marker {
            type Payload = #event;
        }
    }
}

/// Generates a Rust enum with one variant per schema event declared by the entity.
/// Each variant carries the data declared by its schema event fields.
pub(crate) fn entity_event_payload_enum(
    entity: &Entity,
    opts: &Options,
    event_runtime: &Runtime,
) -> Result<TokenStream, Error> {
    let entity_pascal = path_name_pascal(entity.path());
    let enum_ident = raw_ident(format!("{entity_pascal}Event"));
    let docs = doc_attr_or(
        entity.annotations().docs(),
        &format!("Events emitted by `{}` entities.", entity.path()),
    );
    let derives = derive_attr(
        opts.event_derives,
        opts.debug,
        opts.serde,
        opts.serde,
        event_runtime.serde.as_ref(),
    )?;
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
                    let ty =
                        map_data_type(field.ty(), 0, entity.path().namespace(), event_runtime)?;
                    let field_docs = doc_attr(field.annotations().docs());
                    Ok::<_, Error>(quote! { #field_docs #name: #ty })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if fields.is_empty() {
                Ok(quote! { #variant_docs #variant })
            } else {
                Ok(quote! { #variant_docs #variant { #(#fields),* } })
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;
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
    use quent_schema::{
        Annotations, Cardinality, DataType, Field,
        builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, SchemaBuilder},
        test_utils::{entity, event, field, ident, record_type, schema},
    };

    fn event_src(entity: &Entity) -> String {
        let opts = Options {
            debug: false,
            ..Options::default()
        };
        pretty(entity_event_payload_enum(entity, &opts, &crate::tests::runtime()).unwrap())
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
                    id: ::quent_events::Uuid,
                    text: String,
                    n: u32,
                    opt: Option<i32>,
                    list: Vec<String>,
                    rec: SomeRecord,
                    dynrec: ::quent_events::DynamicAttributes,
                    eref: ::quent_events::EntityRef<AnyEntity>,
                    eref_payload: ::quent_events::EntityRef<AnyEntity, u64>
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
                            "nested_lists",
                            DataType::List(Box::new(DataType::List(Box::new(DataType::String)))),
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
                    nested_lists: Vec<Vec<String>>,
                    eref_list: ::quent_events::EntityRef<AnyEntity, Vec<String>>
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
