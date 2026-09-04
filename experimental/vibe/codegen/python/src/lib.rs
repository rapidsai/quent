// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates PyO3 bridges and type stubs from a [`quent_schema::Schema`].

mod common;
mod conversion;
mod stubs;

use std::path::Path;

use common::{
    is_python_identifier, model_path, path_pascal, path_snake, pretty, py_safe, raw_ident,
    rust_path, to_case,
};
use convert_case::Case;
use proc_macro2::{Span, TokenStream};
use quent_constraints::{Report, validate};
use quent_ref_target::RefTargetConstraint;
use quent_schema::{Cardinality, Schema};
use quote::{format_ident, quote};

pub use quent_schema;

/// Configuration for PyO3 bridge generation.
pub struct Options {
    pub module_name: String,
    pub instrumentation_path: String,
    pub runtime_path: String,
    pub io_path: String,
    pub dynamic_attributes_path: String,
    pub exporters: Exporters,
}

/// Exporter constructors generated for a Python module.
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
            module_name: "quent_model".to_owned(),
            instrumentation_path: "instrumentation".to_owned(),
            runtime_path: "quent_instrumentation".to_owned(),
            io_path: "quent_io".to_owned(),
            dynamic_attributes_path: "quent_dynamic_attributes".to_owned(),
            exporters: Exporters::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    pub name: String,
    pub content: String,
}

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
    #[error("generated Python name `{name}` is ambiguous")]
    NameCollision { name: String },
    #[error("Python cannot represent {location}: {reason}")]
    UnsupportedType { location: String, reason: String },
    #[error("invalid generator option `{option}`: {reason}")]
    InvalidOption {
        option: &'static str,
        reason: String,
    },
    #[error("failed to write generated bindings: {0}")]
    Io(#[from] std::io::Error),
}

pub fn emit(schema: &Schema, options: &Options) -> Result<Vec<GeneratedFile>, GenerateError> {
    validate_schema(schema)?;
    validate_options(options)?;
    validate_names(schema)?;
    validate_types(schema)?;
    let instrumentation = parse_path(&options.instrumentation_path)?;
    let runtime = parse_path(&options.runtime_path)?;
    let io = parse_path(&options.io_path)?;
    let dynamic = parse_path(&options.dynamic_attributes_path)?;
    let helpers = helpers(&runtime, &dynamic);
    let context = context(schema, options, &instrumentation, &runtime, &io);
    let entities = schema
        .entities()
        .map(|entity| entity_bindings(schema, entity, &instrumentation, &runtime))
        .collect::<Result<Vec<_>, _>>()?;
    let module = module_registration(schema, options);
    let tokens = quote! {
        #[allow(clippy::needless_borrow)]
        mod __quent_pyo3_bridge {
            #helpers
            #context
            #(#entities)*
            #module
        }
    };
    Ok(vec![GeneratedFile {
        name: "pyo3_bridge.rs".to_owned(),
        content: pretty(tokens)?,
    }])
}

pub fn emit_stubs(schema: &Schema, options: &Options) -> Result<Vec<GeneratedFile>, GenerateError> {
    validate_schema(schema)?;
    validate_options(options)?;
    validate_names(schema)?;
    validate_types(schema)?;
    Ok(stubs::emit(schema, options))
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

fn validate_names(schema: &Schema) -> Result<(), GenerateError> {
    let mut names = [
        "Context",
        "ExporterOptions",
        "Mapping",
        "PathLike",
        "TypedDict",
        "Uuid",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<std::collections::BTreeSet<_>>();
    let mut references = std::collections::BTreeMap::<String, String>::new();
    for record in schema.records() {
        reserve_name(&mut names, format!("{}Dict", path_pascal(record.path())))?;
        validate_python_fields(record.fields().map(|field| field.name().as_ref()))?;
        for field in record.fields() {
            collect_reference_names(field.ty(), &mut references)?;
        }
    }
    for entity in schema.entities() {
        for name in [
            format!("{}Observer", path_pascal(entity.path())),
            format!("{}Handle", path_pascal(entity.path())),
            path_snake(entity.path()),
        ] {
            reserve_name(&mut names, name)?;
        }
        let mut methods = ["uuid".to_owned()]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        for event in entity.events() {
            let method = py_safe(&to_case(event.name(), Case::Snake));
            reserve_name(&mut methods, method.clone())?;
            if event.cardinality() == Cardinality::Once {
                reserve_name(
                    &mut methods,
                    py_safe(&format!("{}_emitted", to_case(event.name(), Case::Snake))),
                )?;
            }
            validate_python_fields(event.fields().map(|field| field.name().as_ref()))?;
            for field in event.fields() {
                collect_reference_names(field.ty(), &mut references)?;
            }
        }
    }
    for name in references.keys() {
        reserve_name(&mut names, name.clone())?;
    }
    Ok(())
}

fn validate_options(options: &Options) -> Result<(), GenerateError> {
    if options.module_name.is_empty()
        || options
            .module_name
            .split('.')
            .any(|part| !is_python_identifier(part) || py_safe(part) != part)
    {
        return Err(GenerateError::InvalidOption {
            option: "module_name",
            reason: "expected non-keyword Python identifiers separated by `.`".to_owned(),
        });
    }
    Ok(())
}

fn validate_types(schema: &Schema) -> Result<(), GenerateError> {
    for record in schema.records() {
        for field in record.fields() {
            validate_type(
                field.ty(),
                &format!("record `{}` field `{}`", record.path(), field.name()),
            )?;
        }
    }
    for entity in schema.entities() {
        for event in entity.events() {
            for field in event.fields() {
                validate_type(
                    field.ty(),
                    &format!(
                        "event `{}.{}` field `{}`",
                        entity.path(),
                        event.name(),
                        field.name()
                    ),
                )?;
            }
        }
    }
    Ok(())
}

fn validate_type(ty: &quent_schema::DataType, location: &str) -> Result<(), GenerateError> {
    use quent_schema::DataType;
    match ty {
        DataType::Option(inner) => {
            if matches!(inner.as_ref(), DataType::Option(_)) {
                return Err(GenerateError::UnsupportedType {
                    location: location.to_owned(),
                    reason: "nested options require distinct `None` and `Some(None)` values"
                        .to_owned(),
                });
            }
            validate_type(inner, location)
        }
        DataType::List(inner) => validate_type(inner, location),
        DataType::EntityRef {
            data: Some(data), ..
        } => validate_type(data, location),
        _ => Ok(()),
    }
}

fn validate_python_fields<'a>(names: impl Iterator<Item = &'a str>) -> Result<(), GenerateError> {
    let mut generated = std::collections::BTreeSet::new();
    for name in names {
        reserve_name(&mut generated, py_safe(&to_case(name, Case::Snake)))?;
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
            let name = stubs::ref_stub_name(data, annotations);
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

fn parse_path(path: &str) -> Result<syn::Path, GenerateError> {
    syn::parse_str(path).map_err(|source| GenerateError::InvalidRustPath {
        path: path.to_owned(),
        source,
    })
}

fn helpers(runtime: &syn::Path, dynamic: &syn::Path) -> TokenStream {
    quote! {
        use pyo3::prelude::*;
        use pyo3::types::{
            PyAny, PyBool, PyBoolMethods, PyFloat, PyFloatMethods, PyInt, PyMapping,
            PyMappingMethods, PyModule, PyString, PyStringMethods, PyTuple,
        };

        #[pyclass(name = "Uuid", frozen, skip_from_py_object)]
        #[derive(Clone)]
        pub struct PyUuid { inner: #runtime::Uuid }

        #[pymethods]
        impl PyUuid {
            pub fn __repr__(&self) -> String { format!("Uuid('{}')", self.inner) }
            pub fn __str__(&self) -> String { self.inner.to_string() }
            pub fn __richcmp__(
                &self,
                other: &Bound<'_, PyAny>,
                op: pyo3::class::basic::CompareOp,
            ) -> PyResult<bool> {
                match op {
                    pyo3::class::basic::CompareOp::Eq => Ok(other
                        .extract::<PyRef<'_, PyUuid>>()
                        .is_ok_and(|other| self.inner == other.inner)),
                    pyo3::class::basic::CompareOp::Ne => Ok(match other
                        .extract::<PyRef<'_, PyUuid>>() {
                            Ok(other) => self.inner != other.inner,
                            Err(_) => true,
                        }),
                    _ => Err(pyo3::exceptions::PyTypeError::new_err(
                        "Uuid only supports equality comparison",
                    )),
                }
            }
            pub fn __hash__(&self) -> isize {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                self.inner.hash(&mut hasher);
                hasher.finish() as isize
            }
        }

        fn __extract_uuid(value: &Bound<'_, PyAny>) -> PyResult<#runtime::Uuid> {
            value.extract::<PyRef<'_, PyUuid>>()
                .map(|value| value.inner)
                .map_err(|_| pyo3::exceptions::PyTypeError::new_err("expected Uuid"))
        }

        fn __extract_dynamic_attributes(
            value: &Bound<'_, PyAny>,
        ) -> PyResult<#runtime::DynamicAttributes> {
            let values = value.cast::<PyMapping>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err("expected mapping for dynamic attributes")
            })?;
            let mut output = #runtime::DynamicAttributes::new();
            for item in values.items()?.iter() {
                let pair = item.cast::<PyTuple>().map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err(
                        "mapping items must contain key-value pairs",
                    )
                })?;
                let key = pair.get_item(0)?;
                let value = pair.get_item(1)?;
                let key = key.cast::<PyString>()
                    .map_err(|_| pyo3::exceptions::PyTypeError::new_err(
                        "dynamic attribute keys must be strings",
                    ))?
                    .to_str()?
                    .to_owned();
                if value.is_none() {
                    output.add(#dynamic::DynamicAttribute::null(key));
                } else if let Ok(value) = value.cast::<PyBool>() {
                    output.add_bool(key, value.is_true());
                } else if let Ok(value) = value.cast::<PyInt>() {
                    if let Ok(value) = value.extract::<i64>() {
                        output.add_i64(key, value);
                    } else {
                        output.add_u64(key, value.extract::<u64>()?);
                    }
                } else if let Ok(value) = value.cast::<PyFloat>() {
                    output.add_f64(key, value.value());
                } else if let Ok(value) = value.cast::<PyString>() {
                    output.add_string(key, value.to_str()?);
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                        "unsupported dynamic attribute value for `{key}`",
                    )));
                }
            }
            Ok(output)
        }

        #[pyfunction]
        pub fn now_v7() -> PyUuid { PyUuid { inner: #runtime::Uuid::now_v7() } }
        #[pyfunction]
        pub fn nil_uuid() -> PyUuid { PyUuid { inner: #runtime::Uuid::nil() } }
    }
}

fn context(
    schema: &Schema,
    options: &Options,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
    io: &syn::Path,
) -> TokenStream {
    let model = model_path(instrumentation, schema.name());
    let context_ty = quote! { #instrumentation::Context<#model> };
    let module_name = &options.module_name;
    let observer_methods = schema.entities().map(|entity| {
        let method = raw_ident(py_safe(&format!("{}_observer", path_snake(entity.path()))));
        let observer = format_ident!("Py{}Observer", path_pascal(entity.path()));
        let entity_ty = rust_path(instrumentation, entity.path(), "");
        quote! {
            pub fn #method(&self) -> PyResult<#observer> {
                let context = self.inner.as_ref().ok_or_else(|| {
                    pyo3::exceptions::PyRuntimeError::new_err(
                        format!("`{}` context is closed", #module_name),
                    )
                })?;
                Ok(#observer { inner: context.observer::<#entity_ty>() })
            }
        }
    });
    let option_variant = options
        .exporters
        .any()
        .then(|| quote! { Options(#io::ExporterOptions), });
    let option_match = options.exporters.any().then(|| {
        quote! { Some(ExporterKind::Options(options)) => <#context_ty>::try_new(options.clone()), }
    });
    let mut exporter_methods = Vec::new();
    if options.exporters.ndjson {
        exporter_methods.push(quote! {
            #[staticmethod]
            pub fn ndjson(output_dir: std::path::PathBuf) -> Self {
                Self::filesystem(#io::FileSystemFormat::Ndjson, output_dir)
            }
        });
    }
    if options.exporters.msgpack {
        exporter_methods.push(quote! {
            #[staticmethod]
            pub fn msgpack(output_dir: std::path::PathBuf) -> Self {
                Self::filesystem(#io::FileSystemFormat::Msgpack, output_dir)
            }
        });
    }
    if options.exporters.postcard {
        exporter_methods.push(quote! {
            #[staticmethod]
            pub fn postcard(output_dir: std::path::PathBuf) -> Self {
                Self::filesystem(#io::FileSystemFormat::Postcard, output_dir)
            }
        });
    }
    if options.exporters.collector {
        exporter_methods.push(quote! {
            #[staticmethod]
            pub fn collector(address: String) -> PyResult<Self> {
                let options = #io::CollectorExporterOptions::try_new(&address)
                    .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))?;
                Ok(Self { inner: ExporterKind::Options(
                    #io::ExporterOptions::Collector(options),
                ) })
            }
        });
    }
    let filesystem_helper = (options.exporters.ndjson
        || options.exporters.msgpack
        || options.exporters.postcard)
        .then(|| {
            quote! {
                fn filesystem(format: #io::FileSystemFormat, output_dir: std::path::PathBuf) -> Self {
                    Self { inner: ExporterKind::Options(#io::ExporterOptions::FileSystem(
                        #io::FileSystemExporterOptions::new(format, output_dir),
                    )) }
                }
            }
        });
    quote! {
        enum ExporterKind { Noop, #option_variant }

        #[pyclass(name = "ExporterOptions", frozen)]
        pub struct PyExporterOptions { inner: ExporterKind }

        impl PyExporterOptions {
            #filesystem_helper
        }

        #[pymethods]
        impl PyExporterOptions {
            #[staticmethod]
            pub fn none() -> Self { Self { inner: ExporterKind::Noop } }
            #(#exporter_methods)*
        }

        #[pyclass(name = "Context")]
        pub struct PyContext { inner: Option<#context_ty> }

        #[pymethods]
        impl PyContext {
            #[new]
            #[pyo3(signature = (options=None))]
            pub fn new(options: Option<PyRef<'_, PyExporterOptions>>) -> PyResult<Self> {
                let result = match options.as_deref().map(|options| &options.inner) {
                    None | Some(ExporterKind::Noop) => <#context_ty>::try_new(#runtime::Noop),
                    #option_match
                };
                let inner = result.map_err(|error| {
                    pyo3::exceptions::PyRuntimeError::new_err(error.to_string())
                })?;
                Ok(Self { inner: Some(inner) })
            }
            #[getter]
            pub fn id(&self) -> PyResult<PyUuid> {
                self.inner.as_ref()
                    .map(|context| PyUuid { inner: context.id() })
                    .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err(
                        format!("`{}` context is closed", #module_name),
                    ))
            }
            pub fn close(&mut self) { self.inner.take(); }
            pub fn __enter__(slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> { slf }
            pub fn __exit__(
                &mut self,
                _exc_type: &Bound<'_, PyAny>,
                _exc_value: &Bound<'_, PyAny>,
                _traceback: &Bound<'_, PyAny>,
            ) { self.close(); }
            #(#observer_methods)*
        }
    }
}

fn entity_bindings(
    schema: &Schema,
    entity: &quent_schema::Entity,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
) -> Result<TokenStream, GenerateError> {
    let name = path_pascal(entity.path());
    let observer = format_ident!("Py{name}Observer");
    let handle = format_ident!("Py{name}Handle");
    let observer_export = format!("{name}Observer");
    let handle_export = format!("{name}Handle");
    let entity_ty = rust_path(instrumentation, entity.path(), "");
    let methods = entity
        .events()
        .map(|event| {
            let model_method = raw_ident(to_case(event.name(), Case::Snake));
            let method = raw_ident(py_safe(&to_case(event.name(), Case::Snake)));
            let params = event
                .fields()
                .map(|field| {
                    let name = raw_ident(py_safe(&to_case(field.name(), Case::Snake)));
                    quote! { #name: &Bound<'_, PyAny> }
                })
                .collect::<Vec<_>>();
            let bindings = event
                .fields()
                .map(|field| {
                    let name = raw_ident(py_safe(&to_case(field.name(), Case::Snake)));
                    let value = conversion::convert(
                        schema,
                        field.ty(),
                        quote! { #name },
                        instrumentation,
                        runtime,
                    )?;
                    Ok::<_, GenerateError>(quote! { let #name = #value; })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let args = event
                .fields()
                .map(|field| raw_ident(py_safe(&to_case(field.name(), Case::Snake))))
                .collect::<Vec<_>>();
            let signature = (!args.is_empty()).then(|| quote! {
                #[pyo3(signature = (*, #(#args),*))]
            });
            let event_method = match event.cardinality() {
                Cardinality::Once => quote! {
                    #signature
                    #[allow(clippy::too_many_arguments)]
                    pub fn #method(&mut self, #(#params),*) -> PyResult<()> {
                        #(#bindings)*
                        self.inner.#model_method(#(#args),*)
                            .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))
                    }
                },
                Cardinality::Multi => quote! {
                    #signature
                    #[allow(clippy::too_many_arguments)]
                    pub fn #method(&self, #(#params),*) -> PyResult<()> {
                        #(#bindings)*
                        self.inner.#model_method(#(#args),*)
                            .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))
                    }
                },
            };
            let emitted = (event.cardinality() == Cardinality::Once).then(|| {
                let emitted = raw_ident(py_safe(&format!(
                    "{}_emitted",
                    to_case(event.name(), Case::Snake)
                )));
                let model_emitted = raw_ident(format!(
                    "{}_emitted",
                    to_case(event.name(), Case::Snake)
                ));
                quote! {
                    pub fn #emitted(&self) -> bool { self.inner.#model_emitted() }
                }
            });
            Ok::<_, GenerateError>(quote! { #event_method #emitted })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! {
        #[pyclass(name = #observer_export)]
        pub struct #observer { inner: #instrumentation::Observer<#entity_ty> }

        #[pymethods]
        impl #observer {
            #[pyo3(signature = (id=None))]
            pub fn create(&self, id: Option<PyRef<'_, PyUuid>>) -> #handle {
                let inner = match id {
                    Some(id) => self.inner.handle_with_id(id.inner),
                    None => self.inner.handle(),
                };
                #handle { inner }
            }
        }

        #[pyclass(name = #handle_export)]
        pub struct #handle { inner: #instrumentation::Handle<#entity_ty> }

        #[pymethods]
        impl #handle {
            #[getter]
            pub fn uuid(&self) -> PyUuid { PyUuid { inner: self.inner.uuid() } }
            #(#methods)*
        }
    })
}

fn module_registration(schema: &Schema, options: &Options) -> TokenStream {
    let rust_name = raw_ident(options.module_name.replace('.', "_"));
    let export_name = options
        .module_name
        .rsplit('.')
        .next()
        .unwrap_or(&options.module_name);
    let export_name = syn::LitStr::new(export_name, Span::call_site());
    let observers = schema
        .entities()
        .map(|entity| format_ident!("Py{}Observer", path_pascal(entity.path())));
    let handles = schema
        .entities()
        .map(|entity| format_ident!("Py{}Handle", path_pascal(entity.path())));
    quote! {
        #[pymodule(name = #export_name)]
        pub fn #rust_name(module: &Bound<'_, PyModule>) -> PyResult<()> {
            module.add_function(wrap_pyfunction!(now_v7, module)?)?;
            module.add_function(wrap_pyfunction!(nil_uuid, module)?)?;
            module.add_class::<PyUuid>()?;
            module.add_class::<PyExporterOptions>()?;
            module.add_class::<PyContext>()?;
            #(module.add_class::<#observers>()?;)*
            #(module.add_class::<#handles>()?;)*
            Ok(())
        }
    }
}

pub fn write_generated_files(
    files: &[GeneratedFile],
    directory: impl AsRef<Path>,
) -> Result<(), GenerateError> {
    for file in files {
        let path = directory.as_ref().join(&file.name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &file.content)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
