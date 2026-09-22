// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates Rust event types and an optional instrumentation surface from a
//! [`quent_schema::Schema`].
//!
//! The usual workflow is build-time generation:
//!
//! 1. From your crate's build script, call [`generate`] with `out_dir` set to
//!    the directory Cargo provides via the `OUT_DIR` environment variable; it
//!    writes the generated source there.
//! 2. Pull that file into your crate's source at compile time with the
//!    `include!` macro.
//!
//! # Example
//!
//! In your crate's `build.rs`:
//!
//! ```ignore
//! use quent_instrumentation_build::{Options, generate};
//!
//! let schema = todo!();
//! let opts = Options::default();
//! generate(&schema, &opts)?;
//! ```
//!
//! Then, anywhere in your crate's source:
//!
//! ```ignore
//! pub mod demo {
//!     include!(concat!(env!("OUT_DIR"), "/demo.rs"));
//! }
//! ```
//!
//! # Restrictions
//!
//! The schema does not limit how many events an entity declares, but the
//! instrumentation surface caps once-cardinality events on non-FSM entities
//! ([`Cardinality::Once`](quent_schema::Cardinality::Once)) events at 64 per
//! entity; beyond that, generation fails with
//! [`GenerateError::TooManyOnceEvents`].
//!
//! Serde derives are opt-in through [`Options::serde`]. The generated crate
//! must also depend on `serde` with its derive feature and enable the matching
//! runtime crate's `serde` feature.

mod common;
mod data_type;
mod events;
mod model;
mod namespace;
mod nvtx;
mod records;
mod runtime;

use std::path::PathBuf;

use convert_case::Case;
use quent_constraints::{BaseConstraintsError, Report};
use quent_fsm::{FsmConstraint, FsmError};
use quent_os::{OsConstraint, OsError};
use quent_ref_target::{RefTargetConstraint, RefTargetError};
use quent_ref_tree::{RefTreeConstraint, RefTreeError};
use quent_schema::{Entity, Path, Schema};
use quote::quote;

/// Options controlling event and instrumentation source generation.
pub struct Options {
    /// Add handles, observers, and context integration to the event types.
    pub instrumentation: bool,

    /// Derive [`Debug`](std::fmt::Debug) on generated event and record types.
    pub debug: bool,

    /// Derive `serde::Serialize` and `serde::Deserialize` on generated event
    /// and record types.
    ///
    pub serde: bool,

    /// Derives applied to every generated event payload enum.
    ///
    /// Use [`Self::debug`] and [`Self::serde`] for the built-in derives.
    pub event_derives: &'static [&'static str],

    /// Derives applied to every generated record struct.
    ///
    /// Use [`Self::debug`] and [`Self::serde`] for the built-in derives.
    pub record_derives: &'static [&'static str],

    /// Directory the generated file is written into.
    pub out_dir: PathBuf,

    /// File name to write; defaults to `<schema name>.rs` (lowercased) when
    /// `None`.
    pub file_name: Option<String>,

    /// Emit model-wide umbrella event enums and implement the umbrella
    /// capability for the generated model.
    ///
    /// No namespace enum is emitted without entity events, except at the root.
    pub umbrella_event: bool,

    /// Cargo package providing the analyzer for this model.
    pub analyzer_package: Option<String>,

    /// Generate collector dispatch for the model context.
    ///
    /// Requires [`Self::serde`]. The generated crate must expose a `collector`
    /// feature that enables `quent-instrumentation/io-collector`.
    pub collector_sink: bool,

    /// Install live NVTX capture when the schema contains the canonical NVTX
    /// extension.
    ///
    /// This is disabled by default. Enabling it requires live instrumentation
    /// and a generated-crate dependency on `nvtx-injection`. Live capture is
    /// supported on 64-bit Linux; schema generation and stored-event analysis
    /// remain portable while this option is disabled.
    pub nvtx_capture: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            instrumentation: true,
            debug: true,
            serde: false,
            event_derives: Default::default(),
            record_derives: Default::default(),
            out_dir: PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()),
            file_name: None,
            umbrella_event: false,
            analyzer_package: None,
            collector_sink: false,
            nvtx_capture: false,
        }
    }
}

impl Options {
    pub(crate) fn event_runtime(&self) -> proc_macro2::TokenStream {
        if self.instrumentation {
            quote! { ::quent_instrumentation }
        } else {
            quote! { ::quent_events }
        }
    }
}

/// An error from generating event or instrumentation source.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("base schema validation failed: {0}")]
    InvalidSchema(#[from] BaseConstraintsError),
    #[error("fsm validation failed: {0}")]
    InvalidFsm(#[from] FsmError),
    #[error("OS schema validation failed: {0}")]
    InvalidOs(#[from] OsError),
    #[error("reference target validation failed: {0}")]
    InvalidRefTarget(#[from] RefTargetError),
    #[error("reference tree validation failed: {0}")]
    InvalidRefTree(#[from] RefTreeError),
    #[error("NVTX schema validation failed: {0}")]
    InvalidNvtx(#[from] nvtx_schema::NvtxError),
    #[error("invalid derive path {derive:?}")]
    InvalidDerive {
        /// The offending derive entry.
        derive: String,
        /// The underlying parse error.
        source: syn::Error,
    },
    #[error("generated code did not form a valid Rust file")]
    InvalidGeneratedCode(#[source] syn::Error),
    #[error(
        "entity `{entity}` declares {count} once-events, exceeding the maximum of {max}",
        max = crate::runtime::MAX_ONCE_EVENTS
    )]
    TooManyOnceEvents {
        /// The offending entity.
        entity: Path,
        /// The number of once-cardinality events the entity declares.
        count: usize,
    },
    #[error("generated observer type `{generated}` conflicts with schema type `{schema_path}`")]
    GeneratedTypeCollision {
        /// The generated Rust type name.
        generated: String,
        /// The schema type whose generated name conflicts.
        schema_path: Path,
    },
    #[error("`collector_sink` requires serde generation")]
    CollectorSinkRequiresSerde,
    #[error("`nvtx_capture` requires instrumentation generation")]
    NvtxCaptureRequiresInstrumentation,
    #[error(
        "NVTX live capture process `{entity}` is an FSM; live capture requires an ordinary entity handle so its identity event can return a capture error"
    )]
    NvtxCaptureFsmProcessUnsupported {
        /// The bound OS-process entity.
        entity: Path,
    },
    #[error("field type nesting exceeds the maximum depth of {max}")]
    TypeNestingTooDeep { max: usize },
    #[error("failed to write generated file")]
    Io(#[from] std::io::Error),
}

pub struct GenerateInfo {
    pub path: PathBuf,
    pub warnings: Vec<String>,
}

/// Validates the schema requirements shared by generated event models.
///
/// Returns constraint names without registered validators as warnings.
pub fn validate_schema(schema: &Schema) -> Result<Vec<String>, GenerateError> {
    let Report {
        base_constraints,
        unregistered_constraints,
        results,
    } = quent_constraints::validate::<(
        RefTargetConstraint,
        RefTreeConstraint,
        FsmConstraint,
        OsConstraint,
        nvtx_schema::NvtxConstraint,
    )>(schema);

    base_constraints?;
    let (ref_target, ref_tree, fsm, os, nvtx) = results;
    ref_target?;
    ref_tree?;
    fsm?;
    os?;
    nvtx?;
    Ok(unregistered_constraints)
}

/// Returns the model path generated for `schema` relative to the generated module root.
pub fn generated_model_path(schema: &Schema) -> proc_macro2::TokenStream {
    let model = common::raw_ident(common::to_case(schema.name(), Case::Pascal));
    quote! { #model }
}

/// Returns the entity marker path generated relative to the generated module root.
pub fn generated_entity_path(entity: &Entity) -> proc_macro2::TokenStream {
    common::relative_type_path(entity.path(), &[], "")
}

/// Returns the entity event path generated relative to the generated module root.
pub fn generated_entity_event_path(entity: &Entity) -> proc_macro2::TokenStream {
    common::relative_type_path(entity.path(), &[], "Event")
}

/// Generate event source and, when enabled, instrumentation source for `schema`.
pub fn generate(schema: &Schema, opts: &Options) -> Result<GenerateInfo, GenerateError> {
    let warnings = validate_schema(schema)?;

    let file_name = opts
        .file_name
        .clone()
        .unwrap_or_else(|| format!("{}.rs", schema.name().to_string().to_lowercase()));
    let path = opts.out_dir.join(file_name);
    std::fs::write(&path, generate_str_unvalidated(schema, opts)?)?;
    Ok(GenerateInfo { path, warnings })
}

/// Return event source and, when enabled, instrumentation source for `schema`.
///
/// # Errors
///
/// Returns [`GenerateError`] if schema validation fails, a generated observer
/// type conflicts with a schema type, a field type exceeds the supported
/// nesting depth, a derive entry is not a parseable Rust path, or the generated
/// code is not valid Rust.
pub fn generate_str(schema: &Schema, opts: &Options) -> Result<String, GenerateError> {
    validate_schema(schema)?;
    generate_str_unvalidated(schema, opts)
}

fn generate_str_unvalidated(schema: &Schema, opts: &Options) -> Result<String, GenerateError> {
    if opts.collector_sink && !opts.serde {
        return Err(GenerateError::CollectorSinkRequiresSerde);
    }
    // Resolve capture even for event-only generation so an explicitly
    // incompatible option combination fails instead of silently emitting a
    // platform guard with no live instrumentation adapter.
    let _ = nvtx::capture_config(schema, opts)?;
    let namespaces = namespace::Namespace::root(schema);

    let reexports = if opts.instrumentation {
        runtime::reexports()
    } else {
        events::reexports()
    };
    let entity_types = opts.instrumentation.then(|| runtime::entity_types(schema));
    let types = generate_namespace(schema, opts, &namespaces)?;
    let nvtx_bindings = nvtx::generate_bindings(schema, opts)?;
    let observable = opts
        .instrumentation
        .then(|| runtime::generate_model(schema, &namespaces, opts))
        .transpose()?;
    let file = syn::parse2::<syn::File>(quote! {
        #reexports
        #entity_types
        #types
        #nvtx_bindings
        #observable
    })
    .map_err(GenerateError::InvalidGeneratedCode)?;
    Ok(prettyplease::unparse(&file))
}

fn generate_namespace(
    schema: &Schema,
    opts: &Options,
    namespace: &namespace::Namespace<'_>,
) -> Result<proc_macro2::TokenStream, GenerateError> {
    let records = namespace
        .records()
        .iter()
        .map(|record| records::record_struct(record, opts))
        .collect::<Result<Vec<_>, _>>()?;
    let events = namespace
        .entities()
        .iter()
        .map(|entity| events::entity_event_enum(entity, opts))
        .collect::<Result<Vec<_>, _>>()?;
    let entity_types = namespace
        .entities()
        .iter()
        .map(|entity| events::entity_types(entity, opts))
        .collect::<Vec<_>>();
    let runtime = if opts.instrumentation {
        namespace
            .entities()
            .iter()
            .map(|entity| runtime::entity_runtime_types(schema, entity, opts))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let children = namespace
        .children()
        .iter()
        .map(|child| {
            let segment = child
                .path()
                .last()
                .expect("child namespaces extend their parent");
            let module = common::module_ident(segment);
            let contents = generate_namespace(schema, opts, child)?;
            Ok::<_, GenerateError>(quote! {
                pub mod #module {
                    #contents
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let observer_storage = if opts.instrumentation {
        runtime::observer_storage(schema, namespace)?
    } else {
        quote! {}
    };
    let model = model::generate(schema, namespace, opts)?;
    Ok(quote! {
        #(#records)*
        #(#events)*
        #(#entity_types)*
        #(#runtime)*
        #(#children)*
        #model
        #observer_storage
    })
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use quent_constraints::Constraint;
    use quent_fsm::{FsmEntityBuilder, StateDecl};
    use quent_ref_target::RefTargetConstraint;
    use quent_schema::builder::AnnotationsBuilder;
    use quent_schema::builder::SchemaBuilder;
    use quent_schema::test_utils::{entity, event, field, path, record, record_type};
    use quent_schema::{Annotations, DataType};

    fn fsm_state(name: &str, to: &[&str], initial: bool) -> StateDecl {
        StateDecl {
            name: name.parse().unwrap(),
            attributes: vec![],
            to: to.iter().map(|target| target.parse().unwrap()).collect(),
            initial,
        }
    }

    #[test]
    fn generates_event_only_umbrella_without_instrumentation() {
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity("Query", [event("created", [])]))
            .build()
            .unwrap();
        let opts = Options {
            instrumentation: false,
            umbrella_event: true,
            ..Options::default()
        };

        let source = generate_str(&schema, &opts).unwrap();

        assert!(source.contains("impl ::quent_events::ModelEvents for Demo"));
        assert!(source.contains("pub enum DemoEvent"));
        assert!(!source.contains("quent_instrumentation"));
        assert!(!source.contains("pub struct Handle"));
        assert!(!source.contains("Observers"));
    }

    #[test]
    fn registers_fsm_validation_while_preserving_unknown_warnings() {
        let annotations = AnnotationsBuilder::new()
            .with_constraint("example.unknown.v1", None)
            .build()
            .unwrap();
        let entity = FsmEntityBuilder::new(path("Query"))
            .with_annotations(annotations)
            .with_states([
                fsm_state("submitted", &["ready"], true),
                fsm_state("ready", &[], false),
            ])
            .build()
            .unwrap();
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity)
            .build()
            .unwrap();

        assert_eq!(
            validate_schema(&schema).unwrap(),
            vec!["example.unknown.v1"]
        );
    }

    #[test]
    fn invalid_fsm_is_a_generation_error() {
        let annotations = AnnotationsBuilder::new()
            .with_constraint(
                FsmConstraint::NAME,
                Some(r#"{"initial_state":"missing","transitions":[]}"#.to_string()),
            )
            .build()
            .unwrap();
        let entity = quent_schema::builder::EntityBuilder::new(path("Query"))
            .with_event(event("ready", [field("seq", DataType::U16)]))
            .with_annotations(annotations)
            .build()
            .unwrap();
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity)
            .build()
            .unwrap();

        assert!(matches!(
            validate_schema(&schema),
            Err(GenerateError::InvalidFsm(_))
        ));
        assert!(matches!(
            generate_str(&schema, &Options::default()),
            Err(GenerateError::InvalidFsm(_))
        ));
    }

    #[test]
    fn built_in_derive_path_spellings_are_deduplicated() {
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_record(record("Meta", []))
            .with_entity(entity("Query", [event("created", [])]))
            .build()
            .unwrap();
        let opts = Options {
            instrumentation: false,
            debug: true,
            serde: true,
            event_derives: &[
                "Debug",
                "std::fmt::Debug",
                "::core::fmt::Debug",
                "serde::Serialize",
                "::serde::Serialize",
            ],
            record_derives: &[
                "Debug",
                "core::fmt::Debug",
                "::std::fmt::Debug",
                "serde::Deserialize",
                "::serde::Deserialize",
            ],
            ..Options::default()
        };

        let source = generate_str(&schema, &opts).unwrap();

        let derives = "#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]";
        assert_eq!(source.matches(derives).count(), 2);
    }

    #[test]
    fn places_entity_types_in_path_modules() {
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity("Foo::Query", [event("event", [])]))
            .build()
            .unwrap();

        let source = generate_str(&schema, &Options::default()).unwrap();
        assert!(source.contains("pub mod foo"));
        assert!(source.contains("pub enum QueryEvent"));
        assert!(!source.contains("pub type Observer"));
        assert!(source.contains("pub struct Handle<"));
        assert!(!source.contains("pub struct FsmHandle<"));
        assert!(
            source.contains(
                "E: ::quent_instrumentation::InstrumentedEntity<Context = Context<Demo>>"
            )
        );
        assert!(source.contains("impl super::Handle<Query>"));
        assert!(source.contains("impl ::quent_instrumentation::InstrumentedEntity for Query"));
        assert!(source.contains("impl ::quent_instrumentation::events::Entity for Query"));
        assert!(source.contains("type Context = super::Context<super::Demo>"));
        assert!(source.contains("pub struct DemoObservers"));
        assert!(source.contains("struct FooObservers"));
        assert!(source.contains("foo_observers: foo::FooObservers"));
        assert!(source.contains("query_observer: ::quent_instrumentation::Observer<Query>"));
        assert!(source.contains(
            "impl ::quent_instrumentation::ObserverProvider<foo::Query> for DemoObservers"
        ));
        assert!(source.contains(r#"const NAME: &'static str = "Foo::Query""#));
        assert!(!source.contains("foo_query_observer"));
    }

    #[test]
    fn separates_types_with_colliding_flattened_paths() {
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_record(record("Foo::BarBaz", []))
            .with_record(record("FooBar::Baz", []))
            .build()
            .unwrap();

        let source = generate_str(&schema, &Options::default()).unwrap();
        assert!(source.contains("pub mod foo"));
        assert!(source.contains("pub struct BarBaz"));
        assert!(source.contains("pub mod foo_bar"));
        assert!(source.contains("pub struct Baz"));
    }

    #[test]
    fn rejects_observer_type_collisions() {
        let conflicting_path = path("Foo::FooObservers");
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_record(record("Foo::FooObservers", []))
            .with_entity(entity("Foo::Query", [event("event", [])]))
            .build()
            .unwrap();

        assert!(matches!(
            generate_str(&schema, &Options::default()),
            Err(GenerateError::GeneratedTypeCollision {
                generated,
                schema_path,
            }) if generated == "FooObservers" && schema_path == conflicting_path
        ));
    }

    #[test]
    fn does_not_merge_namespaces_that_share_a_rust_name() {
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_record(record("FooBar::First", []))
            .with_record(record("foo_bar::Second", []))
            .build()
            .unwrap();

        let source = generate_str(&schema, &Options::default()).unwrap();
        assert_eq!(source.matches("pub mod foo_bar").count(), 2);
    }

    #[test]
    fn qualifies_types_across_path_modules() {
        let target_annotations = AnnotationsBuilder::new()
            .with_constraint(RefTargetConstraint::NAME, Some("Foo::Worker".to_string()))
            .build()
            .unwrap();
        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_record(record("Bar::Meta", []))
            .with_record(record("Foo::Parent", []))
            .with_record(record("Foo::Nested::Local", []))
            .with_record(record("Foo::Nested::Child::Value", []))
            .with_record(record("Foo::Sibling::Value", []))
            .with_entity(entity("Foo::Worker", [event("created", [])]))
            .with_entity(entity(
                "Foo::Nested::Task",
                [event(
                    "created",
                    [
                        field("meta", record_type("Bar::Meta")),
                        field("parent", record_type("Foo::Parent")),
                        field("local", record_type("Foo::Nested::Local")),
                        field("child", record_type("Foo::Nested::Child::Value")),
                        field("sibling", record_type("Foo::Sibling::Value")),
                        field(
                            "worker",
                            DataType::EntityRef {
                                data: None,
                                annotations: target_annotations,
                            },
                        ),
                        field(
                            "any",
                            DataType::EntityRef {
                                data: None,
                                annotations: Annotations::default(),
                            },
                        ),
                    ],
                )],
            ))
            .build()
            .unwrap();

        let source = generate_str(&schema, &Options::default()).unwrap();
        assert!(source.contains("meta: super::super::bar::Meta"));
        assert!(source.contains("parent: super::Parent"));
        assert!(source.contains("local: Local"));
        assert!(source.contains("child: child::Value"));
        assert!(source.contains("sibling: super::sibling::Value"));
        assert!(source.contains("worker: ::quent_instrumentation::EntityRef<super::Worker>"));
        assert!(
            source.contains("any: ::quent_instrumentation::EntityRef<super::super::AnyEntity>")
        );
    }
}
