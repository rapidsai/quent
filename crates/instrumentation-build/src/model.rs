// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generation of model markers and optional umbrella event enums.

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_schema::Schema;
use quote::quote;
use syn::Ident;

use crate::common::{
    derive_attr, module_ident, path_name_pascal, raw_ident, relative_type_path, to_case,
};
use crate::namespace::Namespace;
use crate::{GenerateError, Options};

pub(crate) fn generate(
    schema: &Schema,
    namespace: &Namespace<'_>,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let model = generate_model(schema, namespace, opts);
    let umbrella =
        if opts.umbrella_event && (namespace.path().is_empty() || namespace.has_entities()) {
            generate_umbrella(schema, namespace, opts)?
        } else {
            quote! {}
        };

    Ok(quote! {
        #umbrella
        #model
    })
}

fn generate_umbrella(
    schema: &Schema,
    namespace: &Namespace<'_>,
    opts: &Options,
) -> Result<TokenStream, GenerateError> {
    let event = event_ident(schema, namespace);
    let docs = if namespace.path().is_empty() {
        format!("Events emitted by the `{}` model.", schema.name())
    } else {
        let path = namespace
            .path()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("::");
        format!("Events emitted by entities in the `{path}` namespace.")
    };
    let derives = derive_attr(opts.event_derives, opts.debug, opts.serde, opts.serde)?;

    let entity_variants = namespace.entities().iter().map(|entity| {
        let variant = raw_ident(path_name_pascal(entity.path()));
        let event = raw_ident(format!("{}Event", path_name_pascal(entity.path())));
        quote! { #variant(#event) }
    });
    let child_variants = namespace.children_with_entities().map(|child| {
        let segment = child
            .path()
            .last()
            .expect("child namespaces extend their parent");
        let variant = raw_ident(to_case(segment, Case::Pascal));
        let module = module_ident(segment);
        let child_event = event_ident(schema, child);
        quote! { #variant(#module::#child_event) }
    });

    let entity_conversions = namespace.entities().iter().map(|entity| {
        let variant = raw_ident(path_name_pascal(entity.path()));
        let source = raw_ident(format!("{}Event", path_name_pascal(entity.path())));
        quote! {
            impl ::core::convert::From<#source> for #event {
                fn from(event: #source) -> Self {
                    Self::#variant(event)
                }
            }
        }
    });
    let mut child_conversions = Vec::new();
    for child in namespace.children_with_entities() {
        let segment = child
            .path()
            .last()
            .expect("child namespaces extend their parent");
        let variant = raw_ident(to_case(segment, Case::Pascal));
        let module = module_ident(segment);
        let child_event = event_ident(schema, child);
        let child_event_path = quote! { #module::#child_event };

        child_conversions.push(quote! {
            impl ::core::convert::From<#child_event_path> for #event {
                fn from(event: #child_event_path) -> Self {
                    Self::#variant(event)
                }
            }
        });
        for entity in child.all_entities() {
            let source = relative_type_path(entity.path(), namespace.path(), "Event");
            child_conversions.push(quote! {
                impl ::core::convert::From<#source> for #event {
                    fn from(event: #source) -> Self {
                        Self::#variant(#child_event_path::from(event))
                    }
                }
            });
        }
        for descendant in child.descendants_with_entities() {
            let source = relative_event_path(schema, descendant, namespace);
            child_conversions.push(quote! {
                impl ::core::convert::From<#source> for #event {
                    fn from(event: #source) -> Self {
                        Self::#variant(#child_event_path::from(event))
                    }
                }
            });
        }
    }

    let model_umbrella = if namespace.path().is_empty() {
        let model = raw_ident(to_case(schema.name(), Case::Pascal));
        let runtime = opts.event_runtime();
        quote! {
            impl #runtime::ModelEvents for #model {
                type UmbrellaEvent = #event;
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #[doc = #docs]
        #derives
        pub enum #event {
            #(#entity_variants,)*
            #(#child_variants,)*
        }

        #(#entity_conversions)*
        #(#child_conversions)*
        #model_umbrella
    })
}

fn generate_model(schema: &Schema, namespace: &Namespace<'_>, opts: &Options) -> TokenStream {
    if !namespace.path().is_empty() {
        return quote! {};
    }

    let model = raw_ident(to_case(schema.name(), Case::Pascal));
    let model_name = schema.name().to_string();
    let model_docs = format!("The `{model_name}` model.");
    let runtime = opts.event_runtime();
    let analyzer_package = match opts.analyzer_package.as_deref() {
        Some(package) => quote! {
            fn analyzer_package() -> ::core::option::Option<&'static str> {
                ::core::option::Option::Some(#package)
            }
        },
        None => quote! {},
    };
    quote! {
        #[doc = #model_docs]
        pub struct #model;

        impl #runtime::build_info::ModelSource for #model {
            fn package() -> &'static str {
                env!("CARGO_PKG_NAME")
            }

            fn source() -> #runtime::build_info::BuildInfo {
                #runtime::build_info::source_or_quent(
                    env!("CARGO_PKG_VERSION"),
                    option_env!("QUENT_SOURCE_REMOTE"),
                    option_env!("QUENT_SOURCE_COMMIT"),
                    option_env!("QUENT_SOURCE_BRANCH"),
                    option_env!("QUENT_SOURCE_DIRTY"),
                    option_env!("QUENT_SOURCE_BUILT_AT"),
                )
            }

            #analyzer_package
        }

        impl #runtime::Model for #model {
            const NAME: &'static str = #model_name;
        }
    }
}

fn event_ident(schema: &Schema, namespace: &Namespace<'_>) -> Ident {
    let name = namespace.path().last().unwrap_or_else(|| schema.name());
    raw_ident(format!("{}Event", to_case(name, Case::Pascal)))
}

fn relative_event_path(
    schema: &Schema,
    namespace: &Namespace<'_>,
    ancestor: &Namespace<'_>,
) -> TokenStream {
    let modules = namespace.path()[ancestor.path().len()..]
        .iter()
        .map(module_ident);
    let event = event_ident(schema, namespace);
    quote! { #(#modules::)* #event }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pretty;
    use quent_schema::builder::SchemaBuilder;
    use quent_schema::test_utils::{entity, event, record};

    fn options() -> Options {
        Options {
            umbrella_event: true,
            ..Options::default()
        }
    }

    #[test]
    fn generates_namespace_events_and_transitive_conversions() {
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity("Root", [event("created", [])]))
            .with_entity(entity("Foo::Query", [event("created", [])]))
            .with_entity(entity("Foo::Nested::Task", [event("created", [])]))
            .build()
            .unwrap();
        let namespaces = Namespace::root(&schema);

        let root = pretty(generate(&schema, &namespaces, &options()).unwrap());
        let foo = pretty(generate(&schema, &namespaces.children()[0], &options()).unwrap());

        assert!(root.contains("pub enum DemoEvent"));
        assert!(root.contains("Foo(foo::FooEvent)"));
        assert!(root.contains("impl ::core::convert::From<foo::QueryEvent> for DemoEvent"));
        assert!(root.contains("impl ::core::convert::From<foo::nested::TaskEvent> for DemoEvent"));
        assert!(
            root.contains("impl ::core::convert::From<foo::nested::NestedEvent> for DemoEvent")
        );
        assert!(foo.contains("pub enum FooEvent"));
        assert!(foo.contains("Query(QueryEvent)"));
        assert!(foo.contains("Nested(nested::NestedEvent)"));
    }

    #[test]
    fn skips_record_only_child_namespaces() {
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_record(record("Metadata::Entry", []))
            .build()
            .unwrap();
        let namespaces = Namespace::root(&schema);

        let root = pretty(generate(&schema, &namespaces, &options()).unwrap());
        let metadata = pretty(generate(&schema, &namespaces.children()[0], &options()).unwrap());

        assert!(root.contains("pub enum DemoEvent"));
        assert!(!metadata.contains("MetadataEvent"));
    }

    #[test]
    fn generates_model_without_umbrella_by_default() {
        let schema = SchemaBuilder::try_new("Demo").unwrap().build().unwrap();
        let namespaces = Namespace::root(&schema);

        let root = pretty(generate(&schema, &namespaces, &Options::default()).unwrap());

        assert!(root.contains("pub struct Demo"));
        assert!(root.contains("impl ::quent_instrumentation::Model for Demo"));
        assert!(!root.contains("DemoEvent"));
        assert!(!root.contains("impl ::quent_instrumentation::ModelEvents for Demo"));
    }

    #[test]
    fn generates_analyzer_package_override() {
        let schema = SchemaBuilder::try_new("Demo").unwrap().build().unwrap();
        let namespaces = Namespace::root(&schema);
        let opts = Options {
            analyzer_package: Some("demo-analyzer".to_owned()),
            ..Options::default()
        };

        let root = pretty(generate(&schema, &namespaces, &opts).unwrap());

        assert!(root.contains("::core::option::Option::Some(\"demo-analyzer\")"));
    }
}
