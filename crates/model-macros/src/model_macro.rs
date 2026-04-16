// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `model!` proc macro implementation.
//!
//! Syntax:
//! ```ignore
//! model! {
//!     Simulator {
//!         root: ResourceRoot,
//!         quent_query_engine_model::Engine,
//!         task::Task,
//!         quent_stdlib::Memory,
//!     }
//! }
//! ```
//!
//! Generates `SimulatorModel` (type alias) and `SimulatorEvent` (event enum).

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Path, Token};

struct DefineModelInput {
    name: Ident,
    root: Path,
    components: Vec<Path>,
}

impl Parse for DefineModelInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let content;
        syn::braced!(content in input);

        // First entry must be `root: Path`
        if content.is_empty() {
            return Err(syn::Error::new_spanned(
                name,
                "model! requires at least a root resource group: `root: MyRoot`",
            ));
        }
        let root_kw: Ident = content.parse()?;
        if root_kw != "root" {
            return Err(syn::Error::new_spanned(
                root_kw,
                "first entry must be `root: <RootResourceGroup>`",
            ));
        }
        content.parse::<Token![:]>()?;
        let root: Path = content.parse()?;
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        let mut components = Vec::new();
        while !content.is_empty() {
            components.push(content.parse::<Path>()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(DefineModelInput {
            name,
            root,
            components,
        })
    }
}

/// Extract the last segment of a path as an Ident.
fn last_segment(path: &Path) -> Ident {
    path.segments.last().unwrap().ident.clone()
}

/// Given a path like `foo::bar::Baz`, construct `foo::bar::BazObserver`.
fn observer_type_path(path: &Path) -> Path {
    let mut obs_path = path.clone();
    if let Some(last) = obs_path.segments.last_mut() {
        last.ident = format_ident!("{}Observer", last.ident);
    }
    obs_path
}

/// Given a path like `foo::bar::Baz`, construct `foo::bar::BazEvent`.
fn event_type_path(path: &Path) -> Path {
    let mut event_path = path.clone();
    if let Some(last) = event_path.segments.last_mut() {
        last.ident = format_ident!("{}Event", last.ident);
    }
    event_path
}

/// Build a nested tuple type from a list of paths, chunking into groups of 16.
fn nested_tuple(paths: &[Path]) -> TokenStream {
    if paths.len() <= 16 {
        quote! { (#(#paths,)*) }
    } else {
        let chunks: Vec<TokenStream> = paths
            .chunks(16)
            .map(|chunk| quote! { (#(#chunk,)*) })
            .collect();
        quote! { (#(#chunks,)*) }
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let serde_derives = crate::util::serde_derives();
    let serde_crate_attr = crate::util::serde_crate_attr();
    let input: DefineModelInput = syn::parse2(input)?;
    let name = &input.name;

    let model_type = format_ident!("{}Model", name);
    let event_type = format_ident!("{}Event", name);

    let root = &input.root;

    // Root is the first component, followed by the rest
    let mut all_components = vec![input.root.clone()];
    all_components.extend(input.components.iter().cloned());
    let variants: Vec<Ident> = all_components.iter().map(last_segment).collect();

    // Validate no duplicate component names (last path segment)
    {
        let mut seen = std::collections::HashMap::new();
        for (i, variant) in variants.iter().enumerate() {
            let name_str = variant.to_string();
            if let Some(&first_idx) = seen.get(&name_str) {
                let _ = first_idx;
                return Err(syn::Error::new_spanned(
                    &all_components[i],
                    format!(
                        "duplicate component name `{name_str}` — two components resolve to the same event enum variant"
                    ),
                ));
            }
            seen.insert(name_str, i);
        }
    }

    let event_types: Vec<Path> = all_components.iter().map(event_type_path).collect();
    let observer_types: Vec<Path> = all_components.iter().map(observer_type_path).collect();
    let model_tuple = nested_tuple(&all_components);
    let context_type = format_ident!("{}Context", name);
    let quent_reexport = format_ident!("__quent_{}", crate::util::to_snake_case(name));
    let impl_macro_name = format_ident!(
        "__define_{}_instrumentation",
        crate::util::to_snake_case(name)
    );

    let observer_methods: Vec<TokenStream> = variants
        .iter()
        .zip(observer_types.iter())
        .map(|(variant, obs_type)| {
            let method_name = format_ident!("{}_observer", crate::util::to_snake_case(variant));
            let doc_factory = format!("Create an observer for {variant} entities.");
            quote! {
                #[doc = #doc_factory]
                pub fn #method_name(&self) -> #obs_type<#event_type> {
                    #obs_type::new(&self.tx)
                }
            }
        })
        .collect();

    let doc_model = format!("Model type alias for {name}.");
    let doc_event = format!("Events emitted by the {name} model.");
    let doc_context = format!(
        "Instrumentation context for the `{name}` model.\n\
         \n\
         This is the entry point for instrumentation. Create one with \
         [`Self::try_new()`], then call the `*_observer()` methods to get \
         observers for each model component."
    );
    let doc_try_new = format!(
        "Create a new {name} instrumentation context.\n\
         \n\
         # Arguments\n\
         * `id` — unique identifier for this context instance (typically `Uuid::now_v7()`). \
         Use the same ID for the root resource group.\n\
         * `exporter` — optional exporter configuration (e.g., ndjson, msgpack). \
         Pass `None` for a no-op context that discards events."
    );

    let output = quote! {
        #[doc = #doc_model]
        pub type #model_type = quent_model::Model<#model_tuple>;

        #[doc = #doc_event]
        #[derive(#serde_derives)]
        #serde_crate_attr
        pub enum #event_type {
            #(#variants(#event_types),)*
        }

        #(
            impl From<#event_types> for #event_type {
                fn from(e: #event_types) -> Self {
                    #event_type::#variants(e)
                }
            }
        )*

        const _: () = {
            assert!(
                <#root as quent_model::ResourceGroup>::IS_ROOT,
                "the `root:` component must be annotated with #[resource_group(root)]"
            );
        };

        #[doc(hidden)]
        pub use quent_model as #quent_reexport;

        #[doc(hidden)]
        #[macro_export]
        macro_rules! #impl_macro_name {
            () => {
                #[doc = #doc_context]
                #[doc(alias = "context")]
                pub struct #context_type {
                    _inner: quent_model::Context<#event_type>,
                    tx: quent_model::EventSender<#event_type>,
                }

                impl #context_type {
                    #[doc = #doc_try_new]
                    pub fn try_new(
                        id: quent_model::uuid::Uuid,
                        exporter: Option<quent_model::exporter::ExporterOptions>,
                    ) -> Result<Self, Box<dyn std::error::Error>> {
                        let inner = quent_model::Context::try_new(id, exporter)?;
                        let tx = inner.events_sender();
                        Ok(Self { _inner: inner, tx })
                    }

                    #(#observer_methods)*
                }
            };
        }
    };

    Ok(output)
}

/// Expand the `instrumentation!` proc macro.
///
/// Invokes the hidden callback macro generated by `model!`.
pub fn expand_instrumentation(input: TokenStream) -> syn::Result<TokenStream> {
    let name: Ident = syn::parse2(input)?;
    let impl_macro_name = format_ident!(
        "__define_{}_instrumentation",
        crate::util::to_snake_case(&name)
    );

    Ok(quote! {
        #impl_macro_name!();
    })
}
