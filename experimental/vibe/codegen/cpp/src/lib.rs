// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates CXX bridges from a [`quent_schema::Schema`].

mod common;
mod dynamic_attributes;
mod facade;
mod types;

use std::path::{Component, Path, PathBuf};

use common::{cxx_safe, model_path, path_pascal, path_snake, pretty, raw_ident, to_case};
use convert_case::Case;
use quent_constraints::{Report, validate};
use quent_fsm::{Fsm, FsmConstraint};
use quent_ref_target::RefTargetConstraint;
use quent_schema::Schema;
use quote::quote;

/// Configuration for CXX bridge generation.
pub struct Options {
    /// Base C++ namespace for generated bindings.
    pub namespace: String,
    /// Rust crate name used in generated CXX include paths.
    pub crate_name: String,
    /// Generated bridge directory relative to the bridge crate.
    pub bridge_path: String,
    /// Rust path containing the schema-generated instrumentation module.
    pub instrumentation_path: String,
    /// Rust path of the instrumentation runtime dependency.
    pub runtime_path: String,
    /// Rust path of the I/O dependency.
    pub io_path: String,
    /// Rust path of the dynamic-attributes dependency.
    pub dynamic_attributes_path: String,
    /// Exporter constructors to expose in the generated API.
    pub exporters: Exporters,
    /// Whether generated contexts can capture NVTX events.
    pub nvtx: NvtxSupport,
}

/// Optional NVTX capture support in generated bindings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NvtxSupport {
    /// Preserve the API without NVTX dependencies.
    #[default]
    Disabled,
    /// Allow active exporters to capture NVTX events.
    Enabled,
}

/// Exporter constructors generated for a bridge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Exporters {
    pub ndjson: bool,
    pub msgpack: bool,
    pub postcard: bool,
    pub collector: bool,
}

impl Exporters {
    /// Enables every exporter constructor.
    pub const fn all() -> Self {
        Self {
            ndjson: true,
            msgpack: true,
            postcard: true,
            collector: true,
        }
    }

    const fn any(self) -> bool {
        self.ndjson || self.msgpack || self.postcard || self.collector
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            namespace: "quent".to_owned(),
            crate_name: "quent-bridge".to_owned(),
            bridge_path: "gen".to_owned(),
            instrumentation_path: "instrumentation".to_owned(),
            runtime_path: "quent_instrumentation".to_owned(),
            io_path: "quent_io".to_owned(),
            dynamic_attributes_path: "quent_dynamic_attributes".to_owned(),
            exporters: Exporters::default(),
            nvtx: NvtxSupport::Disabled,
        }
    }
}

/// A generated source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    pub name: String,
    pub content: String,
}

pub use quent_schema;

/// An error produced while generating CXX bindings.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("base schema validation failed: {0}")]
    InvalidSchema(String),
    #[error("reference target validation failed: {0}")]
    InvalidReferenceTarget(String),
    #[error("invalid Rust path `{path}`: {source}")]
    InvalidRustPath { path: String, source: syn::Error },
    #[error("generated Rust source is invalid: {0}")]
    InvalidGeneratedRust(#[from] syn::Error),
    #[error("CXX cannot represent {location}: {reason}")]
    UnsupportedType { location: String, reason: String },
    #[error("generated CXX name `{name}` is ambiguous")]
    NameCollision { name: String },
    #[error("invalid generator option `{option}`: {reason}")]
    InvalidOption {
        option: &'static str,
        reason: String,
    },
    #[error("failed to write generated bindings: {0}")]
    Io(#[from] std::io::Error),
}

/// Generate CXX bridge modules for `schema`.
pub fn emit(schema: &Schema, options: &Options) -> Result<Vec<GeneratedFile>, GenerateError> {
    validate_schema(schema)?;
    validate_options(options)?;
    validate_names(schema, options)?;
    let instrumentation = parse_path(&options.instrumentation_path)?;
    let runtime = parse_path(&options.runtime_path)?;
    let io = parse_path(&options.io_path)?;
    let dynamic = parse_path(&options.dynamic_attributes_path)?;

    let mut files = vec![
        uuid_file(options, &runtime)?,
        dynamic_attributes::file(options, &runtime, &dynamic)?,
        context_file(schema, options, &instrumentation, &runtime, &io)?,
    ];
    for entity in schema.entities() {
        files.push(entity_file(
            schema,
            entity,
            options,
            &instrumentation,
            &runtime,
        )?);
    }
    files.push(facade::emit(schema, options)?);
    Ok(files)
}

fn validate_schema(schema: &Schema) -> Result<(), GenerateError> {
    let Report {
        base_constraints,
        results: (ref_targets, fsms),
        ..
    } = validate::<(RefTargetConstraint, FsmConstraint)>(schema);
    base_constraints.map_err(|error| GenerateError::InvalidSchema(error.to_string()))?;
    ref_targets.map_err(|error| GenerateError::InvalidReferenceTarget(error.to_string()))?;
    fsms.map_err(|error| GenerateError::InvalidSchema(error.to_string()))
}

fn validate_options(options: &Options) -> Result<(), GenerateError> {
    if options.namespace.is_empty()
        || options
            .namespace
            .split("::")
            .any(|part| !is_cxx_identifier(part) || cxx_safe(part) != part)
    {
        return Err(GenerateError::InvalidOption {
            option: "namespace",
            reason: "expected non-keyword C++ identifiers separated by `::`".to_owned(),
        });
    }
    if options.crate_name.is_empty()
        || options
            .crate_name
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
    {
        return Err(GenerateError::InvalidOption {
            option: "crate_name",
            reason: "expected a non-empty Cargo package name".to_owned(),
        });
    }
    let bridge_path = Path::new(&options.bridge_path);
    if bridge_path.as_os_str().is_empty()
        || bridge_path.is_absolute()
        || bridge_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(GenerateError::InvalidOption {
            option: "bridge_path",
            reason: "expected a non-empty relative path without `.` or `..`".to_owned(),
        });
    }
    Ok(())
}

fn validate_names(schema: &Schema, options: &Options) -> Result<(), GenerateError> {
    let mut file_names = ["uuid", "dynamic_attributes", "context"]
        .into_iter()
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    let mut context_methods = std::collections::BTreeSet::new();
    let mut public_records = std::collections::BTreeSet::new();
    let mut public_references = std::collections::BTreeMap::<String, String>::new();

    for record in schema.records() {
        reserve_name(&mut public_records, path_pascal(record.path()))?;
        validate_fields(record.fields().map(|field| field.name().as_ref()))?;
        for field in record.fields() {
            collect_reference_names(field.ty(), &mut public_references)?;
        }
    }

    for entity in schema.entities() {
        reserve_name(&mut file_names, path_snake(entity.path()))?;
        reserve_name(&mut context_methods, path_snake(entity.path()))?;

        let entity_name = path_pascal(entity.path());
        if entity.path().namespace().is_empty()
            && [
                "Uuid",
                "EntityId",
                "Handle",
                "FsmHandle",
                "DynamicFsmHandle",
                "DynamicAttributes",
                "DynamicList",
                "Context",
            ]
            .contains(&entity_name.as_str())
            || entity.path().namespace().is_empty()
                && entity_name == "NvtxCapture"
                && options.nvtx == NvtxSupport::Enabled
            // Schema and NVTX payloads must not share a filesystem/collector stream.
            || options.nvtx == NvtxSupport::Enabled && entity.path() == "NvtxEvent"
        {
            return Err(GenerateError::NameCollision { name: entity_name });
        }
        let mut scope = [
            format!("{entity_name}Id"),
            format!("{entity_name}Observer"),
            format!("{entity_name}Handle"),
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
        let mut fsm_variants = Fsm::try_from_entity(entity)
            .ok()
            .flatten()
            .map(|_| std::collections::BTreeSet::from(["New".to_owned()]));
        let mut methods = ["id".to_owned()]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        if fsm_variants.is_some() {
            methods.insert("into_dynamic".to_owned());
            methods.insert("dynamic_state".to_owned());
            methods.insert("try_into".to_owned());
        }
        for event in entity.events() {
            let method = cxx_safe(&to_case(event.name(), Case::Snake));
            reserve_name(&mut methods, method.clone())?;
            if event.cardinality() == quent_schema::Cardinality::Once {
                reserve_name(
                    &mut methods,
                    cxx_safe(&format!("{}_emitted", to_case(event.name(), Case::Snake))),
                )?;
            }
            if event.fields().next().is_some() {
                let name = to_case(event.name(), Case::Pascal);
                if !is_bridge_type_identifier(&name) {
                    return Err(GenerateError::NameCollision { name });
                }
                reserve_name(&mut scope, name)?;
            }
            if let Some(variants) = &mut fsm_variants {
                reserve_name(variants, to_case(event.name(), Case::Pascal))?;
            }
            validate_fields(event.fields().map(|field| field.name().as_ref()))?;
            for field in event.fields() {
                collect_reference_names(field.ty(), &mut public_references)?;
            }
        }
    }

    for name in public_records {
        if !is_bridge_type_identifier(&name) {
            return Err(GenerateError::NameCollision { name });
        }
    }
    for name in public_references.keys() {
        if !is_bridge_type_identifier(name) {
            return Err(GenerateError::NameCollision { name: name.clone() });
        }
    }
    Ok(())
}

fn validate_fields<'a>(names: impl Iterator<Item = &'a str>) -> Result<(), GenerateError> {
    let mut generated = std::collections::BTreeSet::new();
    for name in names {
        reserve_name(&mut generated, cxx_safe(&to_case(name, Case::Snake)))?;
    }
    Ok(())
}

fn reserve_name(
    names: &mut std::collections::BTreeSet<String>,
    name: String,
) -> Result<(), GenerateError> {
    if names.insert(name.clone()) {
        Ok(())
    } else {
        Err(GenerateError::NameCollision { name })
    }
}

fn collect_reference_names(
    ty: &quent_schema::DataType,
    names: &mut std::collections::BTreeMap<String, String>,
) -> Result<(), GenerateError> {
    use quent_schema::DataType;
    match ty {
        DataType::Option(inner) | DataType::List(inner) => collect_reference_names(inner, names),
        DataType::EntityRef {
            data: Some(data),
            annotations,
        } => {
            let name = facade::reference_name(data, annotations);
            let identity = format!(
                "{:?}:{data:?}",
                quent_ref_target::RefTarget::from_annotations(annotations)
            );
            if names
                .get(&name)
                .is_some_and(|existing| existing != &identity)
            {
                return Err(GenerateError::NameCollision { name });
            }
            names.insert(name, identity);
            collect_reference_names(data, names)
        }
        _ => Ok(()),
    }
}

fn is_cxx_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_bridge_type_identifier(name: &str) -> bool {
    is_cxx_identifier(name)
        && syn::parse_str::<syn::Ident>(name).is_ok()
        && !matches!(name, "Self" | "self" | "super" | "crate")
}

fn parse_path(path: &str) -> Result<syn::Path, GenerateError> {
    syn::parse_str(path).map_err(|source| GenerateError::InvalidRustPath {
        path: path.to_owned(),
        source,
    })
}

fn uuid_file(options: &Options, runtime: &syn::Path) -> Result<GeneratedFile, GenerateError> {
    let namespace = syn::LitStr::new(
        &format!("{}::detail::uuid", options.namespace),
        proc_macro2::Span::call_site(),
    );
    let tokens = quote! {
        #[cxx::bridge(namespace = #namespace)]
        pub mod ffi {
            unsafe extern "C++" {
                include!("rust/cxx.h");
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct UUID {
                pub high_bits: u64,
                pub low_bits: u64,
            }

            extern "Rust" {
                #[cxx_name = "now_v7"]
                fn uuid_now_v7() -> UUID;
                #[cxx_name = "new_nil"]
                fn uuid_new_nil() -> UUID;
                #[cxx_name = "to_string"]
                fn uuid_to_string(id: &UUID) -> String;
                fn uuid_vec_noop(value: &Vec<UUID>);
            }
        }

        fn uuid_now_v7() -> ffi::UUID {
            #runtime::Uuid::now_v7().into()
        }

        fn uuid_new_nil() -> ffi::UUID {
            #runtime::Uuid::nil().into()
        }

        fn uuid_to_string(id: &ffi::UUID) -> String {
            #runtime::Uuid::from(*id).to_string()
        }

        fn uuid_vec_noop(_: &Vec<ffi::UUID>) {}

        impl From<ffi::UUID> for #runtime::Uuid {
            fn from(value: ffi::UUID) -> Self {
                Self::from_u64_pair(value.high_bits, value.low_bits)
            }
        }

        impl From<#runtime::Uuid> for ffi::UUID {
            fn from(value: #runtime::Uuid) -> Self {
                let (high_bits, low_bits) = value.as_u64_pair();
                Self { high_bits, low_bits }
            }
        }
    };
    Ok(GeneratedFile {
        name: "uuid.rs".to_owned(),
        content: pretty(tokens)?,
    })
}

fn context_file(
    schema: &Schema,
    options: &Options,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
    io: &syn::Path,
) -> Result<GeneratedFile, GenerateError> {
    let model = model_path(instrumentation, schema.name());
    let context_ty = quote! { #instrumentation::Context<#model> };
    let detail_namespace = format!("{}::detail", options.namespace);
    let type_id = format!("{detail_namespace}::Context");
    let include = format!("{}/{}/uuid.rs.h", options.crate_name, options.bridge_path);
    let namespace = &detail_namespace;
    let uuid_namespace = format!("{detail_namespace}::uuid");
    let nvtx_enabled = options.nvtx == NvtxSupport::Enabled;
    let nvtx_declaration = if nvtx_enabled {
        "    #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n    enum NvtxCapture { Disabled, Enabled }\n"
    } else {
        ""
    };
    let nvtx_parameter = if nvtx_enabled {
        ", nvtx_capture: NvtxCapture"
    } else {
        ""
    };
    let mut exporter_declarations =
        String::from("        #[Self = \"ExporterOptions\"] fn none() -> Box<ExporterOptions>;\n");
    if options.exporters.ndjson {
        exporter_declarations.push_str(
            "        #[Self = \"ExporterOptions\"] fn ndjson(output_dir: String) -> Box<ExporterOptions>;\n",
        );
    }
    if options.exporters.msgpack {
        exporter_declarations.push_str(
            "        #[Self = \"ExporterOptions\"] fn msgpack(output_dir: String) -> Box<ExporterOptions>;\n",
        );
    }
    if options.exporters.postcard {
        exporter_declarations.push_str(
            "        #[Self = \"ExporterOptions\"] fn postcard(output_dir: String) -> Box<ExporterOptions>;\n",
        );
    }
    if options.exporters.collector {
        exporter_declarations.push_str(
            "        #[Self = \"ExporterOptions\"] fn collector(address: String) -> Result<Box<ExporterOptions>>;\n",
        );
    }
    let ffi = format!(
        r#"#[cxx::bridge(namespace = "{namespace}")]
pub mod ffi {{
    unsafe extern "C++" {{ include!("rust/cxx.h"); }}
    #[namespace = "{uuid_namespace}"]
    unsafe extern "C++" {{
        include!("{include}");
        type UUID = super::super::uuid::ffi::UUID;
    }}
{nvtx_declaration}    extern "Rust" {{
        type ExporterOptions;
{exporter_declarations}        type Context;
        fn create_context(options: Box<ExporterOptions>{nvtx_parameter}) -> Result<Box<Context>>;
        fn id(self: &Context) -> UUID;
    }}
}}
"#
    );
    let option_variant = options
        .exporters
        .any()
        .then(|| quote! { Options(#io::ExporterOptions), });
    let option_match = options.exporters.any().then(|| {
        if nvtx_enabled {
            quote! {
                ExporterKind::Options(options) => {
                    let inner = <#context_ty>::try_new(options.clone())
                        .map_err(|error| error.to_string())?;
                    if nvtx_capture == ffi::NvtxCapture::Enabled {
                        let context_id = inner.id();
                        let runtime = #runtime::ContextInner::try_new(context_id)
                            .map_err(|error| error.to_string())?;
                        let observer = runtime.block_on(
                            runtime.observer::<quent_nvtx_events::NvtxEvent>(&options)
                        ).map_err(|error| error.to_string())?;
                        let capture = quent_nvtx_bridge::Capture::install(
                            context_id, observer
                        ).map_err(|error| error.to_string())?;
                        Ok(Box::new(Context {
                            _nvtx_capture: Some(capture),
                            inner,
                        }))
                    } else {
                        Ok(Box::new(Context {
                            _nvtx_capture: None,
                            inner,
                        }))
                    }
                }
            }
        } else {
            quote! { ExporterKind::Options(options) => <#context_ty>::try_new(options), }
        }
    });
    let mut exporter_methods = Vec::new();
    if options.exporters.ndjson {
        exporter_methods.push(quote! {
            pub fn ndjson(output_dir: String) -> Box<Self> {
                Self::filesystem(#io::FileSystemFormat::Ndjson, output_dir)
            }
        });
    }
    if options.exporters.msgpack {
        exporter_methods.push(quote! {
            pub fn msgpack(output_dir: String) -> Box<Self> {
                Self::filesystem(#io::FileSystemFormat::Msgpack, output_dir)
            }
        });
    }
    if options.exporters.postcard {
        exporter_methods.push(quote! {
            pub fn postcard(output_dir: String) -> Box<Self> {
                Self::filesystem(#io::FileSystemFormat::Postcard, output_dir)
            }
        });
    }
    if options.exporters.collector {
        exporter_methods.push(quote! {
            pub fn collector(address: String) -> Result<Box<Self>, String> {
                let options = #io::CollectorExporterOptions::try_new(&address)
                    .map_err(|error| error.to_string())?;
                Ok(Box::new(Self {
                    inner: ExporterKind::Options(#io::ExporterOptions::Collector(options)),
                }))
            }
        });
    }
    let filesystem_helper = (options.exporters.ndjson
        || options.exporters.msgpack
        || options.exporters.postcard)
        .then(|| {
            quote! {
                fn filesystem(format: #io::FileSystemFormat, output_dir: String) -> Box<Self> {
                    Box::new(Self {
                        inner: ExporterKind::Options(#io::ExporterOptions::FileSystem(
                            #io::FileSystemExporterOptions::new(format, output_dir.into()),
                        )),
                    })
                }
            }
        });
    // Rust drops fields in declaration order: drain the capture worker and flush
    // its observer before releasing the schema context.
    let nvtx_fields = nvtx_enabled.then(|| {
        quote! {
            _nvtx_capture: Option<quent_nvtx_bridge::Capture>,
        }
    });
    let create_context = if nvtx_enabled {
        quote! {
            pub fn create_context(
                options: Box<ExporterOptions>,
                nvtx_capture: ffi::NvtxCapture,
            ) -> Result<Box<Context>, String> {
                match options.inner {
                    ExporterKind::Noop => {
                        let inner = <#context_ty>::try_new(#runtime::Noop)
                            .map_err(|error| error.to_string())?;
                        Ok(Box::new(Context {
                            _nvtx_capture: None,
                            inner,
                        }))
                    }
                    #option_match
                }
            }
        }
    } else {
        quote! {
            pub fn create_context(options: Box<ExporterOptions>) -> Result<Box<Context>, String> {
                let inner = match options.inner {
                    ExporterKind::Noop => <#context_ty>::try_new(#runtime::Noop),
                    #option_match
                }.map_err(|error| error.to_string())?;
                Ok(Box::new(Context { inner }))
            }
        }
    };
    let tokens = quote! {
        enum ExporterKind {
            Noop,
            #option_variant
        }

        pub struct ExporterOptions { inner: ExporterKind }

        impl ExporterOptions {
            pub fn none() -> Box<Self> { Box::new(Self { inner: ExporterKind::Noop }) }
            #(#exporter_methods)*
            #filesystem_helper
        }

        pub struct Context {
            #nvtx_fields
            pub(crate) inner: #context_ty
        }

        unsafe impl cxx::ExternType for Context {
            type Id = cxx::type_id!(#type_id);
            type Kind = cxx::kind::Opaque;
        }

        #create_context

        impl Context {
            pub fn id(&self) -> super::uuid::ffi::UUID { self.inner.id().into() }
        }
    };
    Ok(GeneratedFile {
        name: "context.rs".to_owned(),
        content: format!("{ffi}\n{}", pretty(tokens)?),
    })
}

fn entity_file(
    schema: &Schema,
    entity: &quent_schema::Entity,
    options: &Options,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
) -> Result<GeneratedFile, GenerateError> {
    types::entity_file(schema, entity, options, instrumentation, runtime)
}

/// Write bridge modules and the module include file used by a bridge crate.
///
/// The bridge crate must include `bridge_mod.rs` within one module. The module
/// may have any valid Rust identifier.
pub fn write_bridge_files(
    files: &[GeneratedFile],
    options: &Options,
) -> Result<Vec<PathBuf>, GenerateError> {
    let out_dir = PathBuf::from(
        std::env::var("OUT_DIR")
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::NotFound, error))?,
    );
    write_bridge_files_to(&out_dir, files, options)
}

fn write_bridge_files_to(
    out_dir: &Path,
    files: &[GeneratedFile],
    options: &Options,
) -> Result<Vec<PathBuf>, GenerateError> {
    let generated_dir = out_dir.join(&options.bridge_path);
    std::fs::create_dir_all(&generated_dir)?;
    let expected = files
        .iter()
        .map(|file| std::ffi::OsString::from(&file.name))
        .collect::<std::collections::BTreeSet<_>>();
    prune_files(&generated_dir, &expected, |path| {
        path.extension().is_some_and(|extension| extension == "rs")
            || path.file_name().is_some_and(|name| name == "quent.hpp")
    })?;
    let mut bridge_files = Vec::new();
    let mut modules = String::new();
    for file in files {
        std::fs::write(generated_dir.join(&file.name), &file.content)?;
        if file.name.ends_with(".rs") {
            bridge_files.push(generated_dir.join(&file.name));
            modules.push_str(&bridge_module_declaration(&file.name, &options.bridge_path));
        }
    }
    std::fs::write(out_dir.join("bridge_mod.rs"), modules)?;
    Ok(bridge_files)
}

fn prune_files(
    directory: &Path,
    expected: &std::collections::BTreeSet<std::ffi::OsString>,
    generated: impl Fn(&Path) -> bool,
) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && generated(&entry.path())
            && !expected.contains(&entry.file_name())
        {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn bridge_module_declaration(file_name: &str, bridge_path: &str) -> String {
    let module = raw_ident(file_name.trim_end_matches(".rs"));
    format!("#[path = \"{bridge_path}/{file_name}\"]\npub mod {module};\n")
}

/// Stage generated headers under stable public include paths.
///
/// Call this after `cxx_build::bridges` has generated headers and before the
/// returned C++ build is compiled.
pub fn stage_cxx_headers(options: &Options) -> Result<PathBuf, GenerateError> {
    let out_dir = PathBuf::from(
        std::env::var("OUT_DIR")
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::NotFound, error))?,
    );
    let generated_dir = out_dir.join(&options.bridge_path);
    let public_dir = out_dir
        .join("cxxbridge/include")
        .join(&options.crate_name)
        .join(&options.bridge_path);
    std::fs::create_dir_all(&public_dir)?;
    let expected = std::fs::read_dir(&generated_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .map(|entry| {
            let mut name = entry.file_name();
            name.push(".h");
            name
        })
        .chain(std::iter::once(std::ffi::OsString::from("quent.hpp")))
        .collect::<std::collections::BTreeSet<_>>();
    prune_files(&public_dir, &expected, |path| {
        path.file_name()
            .is_some_and(|name| name == "quent.hpp" || name.to_string_lossy().ends_with(".rs.h"))
    })?;
    for entry in std::fs::read_dir(&generated_dir)? {
        let entry = entry?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "rs")
        {
            let source = cxx_generated_header(&out_dir, &options.crate_name, &entry.path());
            let mut header_name = entry.file_name();
            header_name.push(".h");
            std::fs::copy(source, public_dir.join(header_name))?;
        }
    }
    std::fs::copy(
        generated_dir.join("quent.hpp"),
        public_dir.join("quent.hpp"),
    )?;
    Ok(out_dir.join("cxxbridge/include"))
}

fn cxx_generated_header(out_dir: &Path, crate_name: &str, source: &Path) -> PathBuf {
    let mut relative = PathBuf::new();
    for component in source.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::CurDir => {}
            Component::ParentDir => {
                relative.pop();
            }
            Component::Normal(part) => relative.push(part),
        }
    }
    let mut file_name = relative
        .file_name()
        .expect("generated bridge file")
        .to_owned();
    file_name.push(".h");
    relative.set_file_name(file_name);
    out_dir
        .join("cxxbridge/include")
        .join(crate_name)
        .join(relative)
}

#[cfg(test)]
mod tests;
