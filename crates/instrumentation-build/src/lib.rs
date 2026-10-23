// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates Rust event types and instrumentation from a
//! [`quent_schema::Schema`].
//!
//! The crate including the generated code must depend on
//! `quent-instrumentation`. Add `quent-instrumentation-build` as a build
//! dependency of that crate.
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
//! ## Event type limits
//!
//! For entities without an FSM, generation supports up to
//! [`runtime::MAX_ONCE_EVENTS`]
//! [`Cardinality::Once`](quent_schema::Cardinality::Once) events. Exceeding
//! this limit returns [`GenerateError::TooManyOnceEvents`].
//!
//! Generated FSM handles support up to [`runtime::MAX_FSM_STATES`] states per
//! entity. Exceeding this limit returns [`GenerateError::TooManyFsmStates`].
//!
//! ## Reserved names
//!
//! Schema event names that become the Rust method name `id` are rejected. FSM
//! state names must also avoid existing FSM handle methods, such as `state`,
//! `try_into`, and `as_entity_ref`.
//!
//! Rust type names generated for schema records and entities must not clash
//! with generated observer types in the same namespace, such as `DemoObservers`
//! for a model named `Demo`.
//!
//! ## Serde requirements
//!
//! To generate event types that support Serde, set [`EventOptions::serde`] to
//! `true` and enable the `serde` feature on `quent-instrumentation`.

pub use quent_events_build::Options as EventOptions;
use quent_events_build::{common, namespace};
mod runtime;

use std::path::PathBuf;

use quent_schema::{Identifier, Path, Schema};
use quote::quote;

/// Generates event and instrumentation source for `schema`.
///
/// # Errors
///
/// Returns [`GenerateError`] if schema validation, code generation, or writing
/// the output fails.
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

/// Returns event and instrumentation source for `schema`.
///
/// # Errors
///
/// Returns [`GenerateError`] if schema validation or code generation fails.
pub fn generate_str(schema: &Schema, opts: &Options) -> Result<String, GenerateError> {
    validate_schema(schema)?;
    generate_str_unvalidated(schema, opts)
}

/// Options controlling event and instrumentation source generation.
pub struct Options {
    /// Options for generated event types, records, and model types.
    pub events: EventOptions,

    /// Directory the generated file is written into.
    pub out_dir: PathBuf,

    /// File name to write; defaults to `<schema name>.rs` (lowercased) when
    /// `None`.
    pub file_name: Option<String>,

    /// Cargo package providing the analyzer for this model.
    pub analyzer_package: Option<String>,

    /// Generate collector dispatch for the model context.
    ///
    /// Requires [`Self::events`] to enable [`EventOptions::serde`].
    ///
    /// Requires `io-collector` on the consuming crate's `quent-instrumentation`
    /// dependency.
    pub collector_sink: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            events: EventOptions::default(),
            out_dir: PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()),
            file_name: None,
            analyzer_package: None,
            collector_sink: false,
        }
    }
}

/// An error from generating event or instrumentation source.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error(transparent)]
    Events(#[from] quent_events_build::Error),
    #[error("fsm validation failed: {0}")]
    InvalidFsm(#[from] quent_fsm::FsmError),
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
    #[error(
        "FSM entity `{entity}` declares {count} states, exceeding the maximum of {max}",
        max = crate::runtime::MAX_FSM_STATES
    )]
    TooManyFsmStates {
        /// The offending entity.
        entity: Path,
        /// The number of states the entity declares.
        count: usize,
    },
    #[error("FSM entity `{entity}` state `{state}` generates reserved handle method `{method}`")]
    ReservedFsmHandleMethod {
        /// The offending entity.
        entity: Path,
        /// The state whose generated method conflicts.
        state: Identifier,
        /// The method name reserved by generated FSM handles.
        method: &'static str,
    },
    #[error("generated observer type `{generated}` conflicts with schema type `{schema_path}`")]
    GeneratedTypeCollision {
        /// The generated Rust type name.
        generated: String,
        /// The schema type whose generated name conflicts.
        schema_path: Path,
    },
    #[error("generated handle method `id` conflicts with an event on entity `{entity}`")]
    HandleIdCollision { entity: Path },
    #[error(
        "generated timestamped event method `{method}` conflicts with an event on entity `{entity}`"
    )]
    TimestampedEventMethodCollision {
        /// The offending entity.
        entity: Path,
        /// The generated method name shared by an event and an `_at` companion.
        method: String,
    },
    #[error(
        "generated parameter `timestamp_ns` conflicts with a field of event `{event}` on entity \
         `{entity}`"
    )]
    TimestampParameterCollision {
        /// The offending entity.
        entity: Path,
        /// The event whose field conflicts with the `_at` method parameter.
        event: Identifier,
    },
    #[error("`collector_sink` requires serde generation")]
    CollectorSinkRequiresSerde,
    #[error("failed to write generated file")]
    Io(#[from] std::io::Error),
}

pub struct GenerateInfo {
    pub path: PathBuf,
    pub warnings: Vec<String>,
}

/// Validates base, reference-target, and FSM constraints for instrumentation
/// generation.
///
/// Returns constraint names without registered validators as warnings.
fn validate_schema(schema: &Schema) -> Result<Vec<String>, GenerateError> {
    let report = quent_constraints::validate::<(
        quent_ref_target::RefTargetConstraint,
        quent_fsm::FsmConstraint,
    )>(schema);
    report
        .base_constraints
        .map_err(quent_events_build::Error::from)?;
    let (ref_target, fsm) = report.results;
    ref_target.map_err(quent_events_build::Error::from)?;
    fsm?;
    Ok(report.unregistered_constraints)
}

/// Routes shared event generation through `quent-instrumentation`'s runtime and
/// Serde re-exports.
fn event_runtime() -> quent_events_build::Runtime {
    quent_events_build::Runtime {
        events: syn::parse_quote!(::quent_instrumentation),
        entity_marker: syn::parse_quote!(::quent_instrumentation::events::EntityMarker),
        serde: Some(syn::parse_quote!(::quent_instrumentation::serde)),
    }
}

fn generate_str_unvalidated(schema: &Schema, opts: &Options) -> Result<String, GenerateError> {
    if opts.collector_sink && !opts.events.serde {
        return Err(GenerateError::CollectorSinkRequiresSerde);
    }
    let namespaces = namespace::Namespace::root(schema);
    let reexports = runtime::reexports();
    let entity_types = runtime::entity_types(schema);
    let types = generate_namespace(schema, opts, &namespaces)?;
    let observable = runtime::generate_model(schema, &namespaces, opts.collector_sink);
    let mut file = syn::parse2::<syn::File>(quote! { #reexports #entity_types #types #observable })
        .map_err(GenerateError::InvalidGeneratedCode)?;
    if let Some(package) = opts.analyzer_package.as_deref() {
        let model_source = file
            .items
            .iter_mut()
            .find_map(|item| match item {
                syn::Item::Impl(item)
                    if item.trait_.as_ref().is_some_and(|(path, _)| {
                        path.segments
                            .last()
                            .is_some_and(|segment| segment.ident == "ModelSource")
                    }) =>
                {
                    Some(item)
                }
                _ => None,
            })
            .expect("shared generation emits model source metadata");
        model_source.items.push(syn::parse_quote! {
            fn analyzer_package() -> ::core::option::Option<&'static str> {
                ::core::option::Option::Some(#package)
            }
        });
    }
    Ok(prettyplease::unparse(&file))
}

fn generate_namespace(
    schema: &Schema,
    opts: &Options,
    namespace: &namespace::Namespace<'_>,
) -> Result<proc_macro2::TokenStream, GenerateError> {
    let quent_events_build::NamespaceCode {
        declarations,
        model,
    } = quent_events_build::generate_namespace(schema, &opts.events, namespace, &event_runtime())?;
    let runtime = namespace
        .entities()
        .iter()
        .map(|entity| runtime::entity_runtime_types(schema, entity))
        .collect::<Result<Vec<_>, _>>()?;
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
            Ok::<_, GenerateError>(quote! { pub mod #module { #contents } })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let observer_storage = runtime::observer_storage(schema, namespace)?;
    Ok(quote! { #declarations #(#runtime)* #(#children)* #model #observer_storage })
}

#[cfg(test)]
fn pretty(tokens: proc_macro2::TokenStream) -> String {
    prettyplease::unparse(&syn::parse2::<syn::File>(tokens).expect("tokens form a valid file"))
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use quent_constraints::Constraint;
    use quent_fsm::{FsmConstraint, FsmEntityBuilder, StateDecl};
    use quent_ref_target::RefTargetConstraint;
    use quent_schema::{
        Annotations, DataType,
        builder::{AnnotationsBuilder, SchemaBuilder},
        test_utils::{entity, event, field, path, record, record_type},
    };

    fn fsm_state(name: &str, to: &[&str], initial: bool) -> StateDecl {
        StateDecl {
            name: name.parse().unwrap(),
            attributes: vec![],
            to: to.iter().map(|target| target.parse().unwrap()).collect(),
            initial,
        }
    }

    #[test]
    fn generates_analyzer_package_override() {
        let schema = SchemaBuilder::try_new("Demo").unwrap().build().unwrap();
        let source = generate_str(
            &schema,
            &Options {
                analyzer_package: Some("demo-analyzer".to_owned()),
                ..Options::default()
            },
        )
        .unwrap();
        assert!(source.contains("::core::option::Option::Some(\"demo-analyzer\")"));
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
        assert!(source.contains("pub fn event_at("));
        assert!(source.contains("timestamp_ns: u64"));
        assert!(source.contains("self.inner.emit_once_at::<0>("));
        assert!(source.contains("impl ::quent_instrumentation::InstrumentedEntity for Query"));
        assert!(source.contains("impl ::quent_instrumentation::events::EntityMarker for Query"));
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
    fn rejects_generated_name_collisions() {
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

        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity("Task", [event("id", [])]))
            .build()
            .unwrap();
        assert!(matches!(
            generate_str(&schema, &Options::default()),
            Err(GenerateError::HandleIdCollision { .. })
        ));

        let schema = SchemaBuilder::try_new("Demo")
            .unwrap()
            .with_entity(entity(
                "Task",
                [event("started", []), event("started_at", [])],
            ))
            .build()
            .unwrap();
        assert!(matches!(
            generate_str(&schema, &Options::default()),
            Err(GenerateError::TimestampedEventMethodCollision { method, .. })
                if method == "started_at"
        ));

        for field_name in ["timestamp_ns", "timestampNs"] {
            let schema = SchemaBuilder::try_new("Demo")
                .unwrap()
                .with_entity(entity(
                    "Task",
                    [event("started", [field(field_name, DataType::U64)])],
                ))
                .build()
                .unwrap();
            assert!(matches!(
                generate_str(&schema, &Options::default()),
                Err(GenerateError::TimestampParameterCollision { event, .. })
                    if event == "started"
            ));
        }
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
