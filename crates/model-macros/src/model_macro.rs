// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! `model!` proc macro implementation.
//!
//! Syntax (fields in any order; `name` + `root` required, `entities` optional):
//! ```ignore
//! model! {
//!     name: Simulator,
//!     root: ResourceRoot,
//!     entities: {
//!         quent_query_engine_model::Engine,
//!         task::Task,
//!         quent_stdlib::memory::Memory,
//!     },
//!     analyzer: "my-analyzer", // optional: crate providing the QuentViewer
//! }
//! ```
//!
//! Generates `SimulatorModel` (type alias), `SimulatorEvent` (event enum), and
//! `Simulator` (the model marker carrying provenance).

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Path, Token};

struct DefineModelInput {
    name: Ident,
    root: Path,
    components: Vec<Path>,
    /// Optional `analyzer: "<crate>"`: the cargo package providing this model's
    /// `QuentViewer` entry, recorded in the provenance sidecar.
    analyzer_package: Option<LitStr>,
}

impl Parse for DefineModelInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Labeled `key: value` fields in any order: `name`/`root` required,
        // `entities` optional (a brace-delimited, comma-separated path list).
        let mut name: Option<Ident> = None;
        let mut root: Option<Path> = None;
        let mut components: Option<Vec<Path>> = None;
        let mut analyzer_package: Option<LitStr> = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let dup = || syn::Error::new_spanned(&key, format!("duplicate `{key}`"));
            match key.to_string().as_str() {
                "name" => {
                    if name.replace(input.parse()?).is_some() {
                        return Err(dup());
                    }
                }
                "root" => {
                    if root.replace(input.parse()?).is_some() {
                        return Err(dup());
                    }
                }
                "entities" => {
                    if components.is_some() {
                        return Err(dup());
                    }
                    let content;
                    syn::braced!(content in input);
                    let mut entities = Vec::new();
                    while !content.is_empty() {
                        entities.push(content.parse::<Path>()?);
                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                    components = Some(entities);
                }
                "analyzer" => {
                    if analyzer_package.replace(input.parse()?).is_some() {
                        return Err(dup());
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!(
                            "unknown `model!` field `{other}`; expected `name`, `root`, `entities`, or `analyzer`"
                        ),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let name = name.ok_or_else(|| input.error("`model!` requires a `name: <Ident>` field"))?;
        let root = root.ok_or_else(|| input.error("`model!` requires a `root: <Path>` field"))?;
        Ok(DefineModelInput {
            name,
            root,
            components: components.unwrap_or_default(),
            analyzer_package,
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

    // One observer field per entity, named with the bare entity snake-case name.
    let observer_fields: Vec<Ident> = variants
        .iter()
        .map(|variant| format_ident!("{}", crate::util::to_snake_case(variant)))
        .collect();

    let observer_methods: Vec<TokenStream> = variants
        .iter()
        .zip(observer_types.iter())
        .zip(observer_fields.iter())
        .map(|((variant, obs_type), field)| {
            let method_name = format_ident!("{}_observer", crate::util::to_snake_case(variant));
            let doc_factory = format!("Observer for {variant} entities.");
            quote! {
                #[doc = #doc_factory]
                pub fn #method_name(&self) -> #obs_type {
                    self.#field.clone()
                }
            }
        })
        .collect();

    // Per-entity observer field declarations.
    let observer_field_decls: Vec<TokenStream> = observer_fields
        .iter()
        .zip(observer_types.iter())
        .map(|(field, obs_type)| {
            quote! { #field: #obs_type }
        })
        .collect();

    // One `ingest` arm per entity: match the wire `entity` name against the
    // entity event type's `NAME`, deserialize its `Event`, and route it.
    let ingest_arms: Vec<TokenStream> = event_types
        .iter()
        .zip(observer_fields.iter())
        .map(|(comp_event, field)| {
            quote! {
                if entity == <#comp_event as quent_model::EntityEvent>::NAME {
                    let e: quent_model::Event<#comp_event> =
                        quent_model::ciborium::from_reader(event)?;
                    self.#field.send(e);
                    return Ok(());
                }
            }
        })
        .collect();

    let doc_model = format!("Model type alias for {name}.");
    let doc_event = format!("Event types of the {name} model.");
    let doc_marker = format!(
        "Marker type for the {name} model. Carries the model's provenance via \
         its [`ModelSource`](quent_model::build_info::ModelSource) impl."
    );
    let doc_context = format!(
        "Instrumentation context for `{name}`.\n\
         \n\
         The entry point for instrumentation: create one with \
         [`Self::try_new()`], then call the `*_observer()` methods to get an \
         observer per entity.\n\
         \n\
         # Runtime\n\
         \n\
         Construction and drop are synchronous but block the calling thread \
         while building exporters and flushing them. Call them from outside any \
         async runtime, or from within a **multi-threaded** Tokio runtime. They \
         **panic** on a current-thread runtime (`#[tokio::main(flavor = \
         \"current_thread\")]`), which has no spare thread to make progress \
         while the caller blocks. The `*_observer()` accessors are cheap and \
         have no such restriction."
    );
    let doc_try_new = format!(
        "Create a new {name} instrumentation context.\n\
         \n\
         Builds every entity's exporter, blocking until they are ready. See the \
         [type docs](Self) for the runtime restriction (panics on a \
         current-thread runtime).\n\
         \n\
         # Arguments\n\
         * `exporter` — optional exporter configuration (e.g., ndjson, msgpack). \
         Pass `None` for a no-op context that discards events."
    );

    let doc_import = format!(
        "Reconstruct the [`{event_type}`] stream for a single context directory.\n\
         \n\
         Reads each entity's per-stream subdirectory under `dir` in `format`, \
         deserializes its events, and chains them. The events carry their own \
         timestamps, so the analyzer orders them; this does not sort.\n\
         \n\
         # Assumption\n\
         \n\
         Treats one context directory as the complete telemetry of one model \
         instance: every process of that instance is assumed to build its context \
         with the same id and export through the collector, which centralizes \
         their per-entity streams under that one id. This does not cover the \
         alternative of lazily loading distributed per-node exports on demand \
         with no collector at capture time."
    );

    // Emit a `ModelSource::analyzer_package()` override only when declared.
    let analyzer_package_method = match &input.analyzer_package {
        Some(lit) => quote! {
            fn analyzer_package() -> Option<&'static str> {
                Some(#lit)
            }
        },
        None => quote! {},
    };

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

        #[doc = #doc_marker]
        pub struct #name;

        // Records this model's package and source git so exporters can trace an
        // artifact back to the crate that defines it — including out-of-repo
        // crates, whose own `build.rs` populates `QUENT_SOURCE_*` (in-repo it
        // falls back to quent's git). `env!`/`option_env!` resolve in the crate
        // that invokes `model!`. The type path and name come from `type_name`.
        impl quent_model::build_info::ModelSource for #name {
            fn package() -> &'static str {
                env!("CARGO_PKG_NAME")
            }
            fn source() -> quent_model::build_info::BuildInfo {
                quent_model::build_info::source_or_quent(
                    env!("CARGO_PKG_VERSION"),
                    option_env!("QUENT_SOURCE_REMOTE"),
                    option_env!("QUENT_SOURCE_COMMIT"),
                    option_env!("QUENT_SOURCE_BRANCH"),
                    option_env!("QUENT_SOURCE_DIRTY"),
                    option_env!("QUENT_SOURCE_BUILT_AT"),
                )
            }
            #analyzer_package_method
        }

        impl #name {
            #[doc = #doc_import]
            pub fn import_events(
                dir: &std::path::Path,
                format: quent_model::exporter::FileSystemFormat,
            ) -> quent_model::exporter::ImporterResult<
                Box<dyn Iterator<Item = quent_model::Event<#event_type>>>,
            > {
                let mut streams: Vec<
                    Box<dyn Iterator<Item = quent_model::Event<#event_type>>>,
                > = Vec::new();
                #(
                    {
                        let path =
                            dir.join(<#event_types as quent_model::EntityEvent>::NAME);
                        if path.is_dir() {
                            let importer = quent_model::exporter::create_importer::<#event_types>(
                                &quent_model::exporter::ImporterOptions::FileSystem(
                                    quent_model::exporter::FileSystemImporterOptions {
                                        format,
                                        path,
                                    },
                                ),
                            )?;
                            streams.push(Box::new(importer.map(|e| {
                                quent_model::Event::new(
                                    e.id,
                                    e.timestamp,
                                    #event_type::from(e.data),
                                )
                            })));
                        }
                    }
                )*
                Ok(Box::new(streams.into_iter().flatten()))
            }
        }

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
                    #(#observer_field_decls,)*
                    _inner: quent_model::Context,
                }

                impl #context_type {
                    #[doc = #doc_try_new]
                    pub fn try_new(
                        exporter: Option<quent_model::exporter::ExporterOptions>,
                    ) -> Result<Self, Box<dyn std::error::Error>> {
                        let inner = quent_model::Context::try_new(
                            <#name as quent_model::build_info::ModelSource>::model_info(),
                            exporter,
                        )?;
                        Self::assemble(inner)
                    }

                    /// Build a context that adopts an existing `id` instead of
                    /// generating one — e.g. the collector reproducing a remote
                    /// source's output under that source's id. Same blocking and
                    /// runtime restriction as [`Self::try_new`].
                    pub fn try_with_id(
                        id: quent_model::uuid::Uuid,
                        exporter: Option<quent_model::exporter::ExporterOptions>,
                    ) -> Result<Self, Box<dyn std::error::Error>> {
                        let inner = quent_model::Context::try_with_id(
                            id,
                            <#name as quent_model::build_info::ModelSource>::model_info(),
                            exporter,
                        )?;
                        Self::assemble(inner)
                    }

                    // The single sync/async bridge: build every entity observer
                    // concurrently on the context's runtime, block until all
                    // complete, then assemble. Everything below this `block_on`
                    // is plain async.
                    fn assemble(
                        inner: quent_model::Context,
                    ) -> Result<Self, Box<dyn std::error::Error>> {
                        let ( #(#observer_fields,)* ) = inner.block_on(async {
                            let ( #(#observer_fields,)* ) =
                                quent_model::tokio::try_join!(
                                    #(inner.observer::<#event_types>(),)*
                                )?;
                            Ok::<_, Box<dyn std::error::Error>>(( #(#observer_fields,)* ))
                        })?;
                        Ok(Self {
                            #(#observer_fields: #observer_types::new(#observer_fields),)*
                            _inner: inner,
                        })
                    }

                    /// Identity of this context, generated on construction.
                    pub fn id(&self) -> quent_model::uuid::Uuid {
                        self._inner.id()
                    }

                    #(#observer_methods)*
                }

                // Collector routing, kept out of the context's own API. A
                // collector factory awaits the `*_observer()` accessors to build
                // the observers before any `ingest` call reads them.
                #[cfg(feature = "collector")]
                impl quent_model::CollectorSink for #context_type {
                    fn ingest(
                        &self,
                        entity: &str,
                        event: &[u8],
                    ) -> Result<(), Box<dyn std::error::Error>> {
                        #(#ingest_arms)*
                        Err(format!("unknown entity stream `{entity}`").into())
                    }
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

#[cfg(test)]
mod parse_tests {
    use super::DefineModelInput;

    #[test]
    fn parses_analyzer_field() {
        let input: DefineModelInput = syn::parse_str(
            r#"name: App, root: a::Root, entities: { b::Comp }, analyzer: "my-analyzer""#,
        )
        .unwrap();
        assert_eq!(input.components.len(), 1);
        assert_eq!(
            input.analyzer_package.map(|l| l.value()),
            Some("my-analyzer".to_string())
        );
    }

    #[test]
    fn analyzer_is_optional() {
        let input: DefineModelInput = syn::parse_str("name: App, root: a::Root").unwrap();
        assert!(input.analyzer_package.is_none());
        assert!(input.components.is_empty());
    }
}
