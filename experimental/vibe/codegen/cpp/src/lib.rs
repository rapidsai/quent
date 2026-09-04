// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates CXX bridges from a [`quent_schema::Schema`].

mod common;
mod facade;
mod types;

use std::path::{Component, Path, PathBuf};

use common::{cxx_safe, model_path, path_pascal, path_snake, pretty, to_case};
use convert_case::Case;
use quent_constraints::{Report, validate};
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
    validate_names(schema)?;
    let instrumentation = parse_path(&options.instrumentation_path)?;
    let runtime = parse_path(&options.runtime_path)?;
    let io = parse_path(&options.io_path)?;
    let dynamic = parse_path(&options.dynamic_attributes_path)?;

    let mut files = vec![
        uuid_file(options, &runtime)?,
        dynamic_attributes_file(options, &runtime, &dynamic)?,
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
        results: (ref_targets,),
        ..
    } = validate::<(RefTargetConstraint,)>(schema);
    base_constraints.map_err(|error| GenerateError::InvalidSchema(error.to_string()))?;
    ref_targets.map_err(|error| GenerateError::InvalidReferenceTarget(error.to_string()))
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

fn validate_names(schema: &Schema) -> Result<(), GenerateError> {
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
        let mut scope = [
            format!("{entity_name}Observer"),
            format!("{entity_name}Handle"),
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
        let mut methods = ["id".to_owned(), "uuid".to_owned()]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
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

fn dynamic_attributes_file(
    options: &Options,
    runtime: &syn::Path,
    dynamic: &syn::Path,
) -> Result<GeneratedFile, GenerateError> {
    let namespace = syn::LitStr::new(
        &format!("{}::detail", options.namespace),
        proc_macro2::Span::call_site(),
    );
    let tokens = quote! {
        #[cxx::bridge(namespace = #namespace)]
        pub mod ffi {
            unsafe extern "C++" {
                include!("rust/cxx.h");
            }

            #[derive(Debug)]
            pub enum DynamicAttributeKind { Null, String, I64, U64, F64, Bool }

            #[derive(Debug)]
            pub struct DynamicAttribute {
                pub kind: DynamicAttributeKind,
                pub key: String,
                pub string_value: String,
                pub i64_value: i64,
                pub u64_value: u64,
                pub f64_value: f64,
                pub bool_value: bool,
            }

            #[derive(Debug, Default)]
            pub struct DynamicAttributes {
                pub values: Vec<DynamicAttribute>,
            }

            extern "Rust" {
                fn dynamic_attributes_vec_noop(value: &Vec<DynamicAttributes>);
            }
        }

        fn dynamic_attributes_vec_noop(_: &Vec<ffi::DynamicAttributes>) {}

        impl ffi::DynamicAttributes {
            pub fn into_model(self) -> #runtime::DynamicAttributes {
                let mut output = #runtime::DynamicAttributes::new();
                for value in self.values {
                    match value.kind {
                        ffi::DynamicAttributeKind::Null => {
                            output.add(#dynamic::DynamicAttribute::null(value.key));
                        }
                        ffi::DynamicAttributeKind::String => {
                            output.add_string(value.key, value.string_value);
                        }
                        ffi::DynamicAttributeKind::I64 => {
                            output.add_i64(value.key, value.i64_value);
                        }
                        ffi::DynamicAttributeKind::U64 => {
                            output.add_u64(value.key, value.u64_value);
                        }
                        ffi::DynamicAttributeKind::F64 => {
                            output.add_f64(value.key, value.f64_value);
                        }
                        ffi::DynamicAttributeKind::Bool => {
                            output.add_bool(value.key, value.bool_value);
                        }
                        _ => unreachable!("unknown dynamic attribute kind"),
                    }
                }
                output
            }
        }
    };
    Ok(GeneratedFile {
        name: "dynamic_attributes.rs".to_owned(),
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
        type UUID = crate::bridge::uuid::ffi::UUID;
    }}
    extern "Rust" {{
        type ExporterOptions;
{exporter_declarations}        type Context;
        fn create_context(options: Box<ExporterOptions>) -> Result<Box<Context>>;
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
        quote! { ExporterKind::Options(options) => <#context_ty>::try_new(options), }
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

        pub struct Context { pub(crate) inner: #context_ty }

        unsafe impl cxx::ExternType for Context {
            type Id = cxx::type_id!(#type_id);
            type Kind = cxx::kind::Opaque;
        }

        pub fn create_context(options: Box<ExporterOptions>) -> Result<Box<Context>, String> {
            let inner = match options.inner {
                ExporterKind::Noop => <#context_ty>::try_new(#runtime::Noop),
                #option_match
            }.map_err(|error| error.to_string())?;
            Ok(Box::new(Context { inner }))
        }

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
pub fn write_bridge_files(
    files: &[GeneratedFile],
    options: &Options,
) -> Result<Vec<PathBuf>, GenerateError> {
    let out_dir = PathBuf::from(
        std::env::var("OUT_DIR")
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::NotFound, error))?,
    );
    let generated_dir = out_dir.join(&options.bridge_path);
    std::fs::create_dir_all(&generated_dir)?;
    let mut bridge_files = Vec::new();
    let mut modules = String::new();
    for file in files {
        std::fs::write(generated_dir.join(&file.name), &file.content)?;
        if file.name.ends_with(".rs") {
            bridge_files.push(generated_dir.join(&file.name));
            let module = file.name.trim_end_matches(".rs");
            modules.push_str(&format!(
                "#[path = \"{}/{}\"]\npub mod {};\n",
                options.bridge_path, file.name, module
            ));
        }
    }
    std::fs::write(out_dir.join("bridge_mod.rs"), modules)?;
    Ok(bridge_files)
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
