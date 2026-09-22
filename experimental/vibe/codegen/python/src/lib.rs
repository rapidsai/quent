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
use quent_fsm::{Fsm, FsmConstraint, SEQUENCE_FIELD_NAME};
use quent_ref_target::RefTargetConstraint;
use quent_schema::{Cardinality, Entity, Schema};
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
    let helpers = helpers(options, &runtime, &dynamic);
    let context = context(schema, options, &instrumentation, &runtime, &io);
    let entities = public_entities(schema)
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
        results: (ref_targets, fsms),
        ..
    } = validate::<(RefTargetConstraint, FsmConstraint)>(schema);
    base_constraints.map_err(|error| GenerateError::InvalidSchema(error.to_string()))?;
    ref_targets.map_err(|error| GenerateError::InvalidReferenceTarget(error.to_string()))?;
    fsms.map_err(|error| GenerateError::InvalidSchema(error.to_string()))?;
    nvtx_schema::validated_bindings(schema)
        .map_err(|error| GenerateError::InvalidSchema(error.to_string()))?;
    Ok(())
}

/// Iterate over entities that belong in the language-facing API.
///
/// The canonical NVTX stream is generated for internal capture. Its validated
/// binding identifies the private marker without relying on its display name.
pub(crate) fn public_entities(schema: &Schema) -> impl Iterator<Item = &Entity> {
    let private_entity = nvtx_schema::validated_bindings(schema)
        .expect("schema was validated before generation")
        .map(|bindings| bindings.entity.path().clone());
    schema
        .entities()
        .filter(move |entity| private_entity.as_ref() != Some(entity.path()))
}

/// Whether this schema contains the validated private NVTX capture source.
pub(crate) fn has_nvtx_source(schema: &Schema) -> bool {
    nvtx_schema::validated_bindings(schema)
        .expect("schema was validated before generation")
        .is_some()
}

fn validate_names(schema: &Schema) -> Result<(), GenerateError> {
    let mut names = [
        "Context",
        "ContextClosedError",
        "DynamicAttributes",
        "DynamicAttributeValue",
        "DynamicValue",
        "EventAlreadyEmittedError",
        "ExporterOptions",
        "HandleConsumedError",
        "Iterable",
        "Mapping",
        "PathLike",
        "QuentError",
        "Sequence",
        "SourceActivationError",
        "TypeAlias",
        "TypedDict",
        "uuid",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<std::collections::BTreeSet<_>>();
    if has_nvtx_source(schema) {
        names.insert("SourceCapture".to_owned());
    }
    let mut references = std::collections::BTreeMap::<String, String>::new();
    for record in schema.records() {
        reserve_name(&mut names, format!("{}Dict", path_pascal(record.path())))?;
        reserve_name(&mut names, format!("{}Input", path_pascal(record.path())))?;
        validate_python_fields(record.fields().map(|field| field.name().as_ref()))?;
        for field in record.fields() {
            collect_reference_names(field.ty(), &mut references)?;
        }
    }
    for entity in public_entities(schema) {
        for name in [
            format!("{}Observer", path_pascal(entity.path())),
            format!("{}Handle", path_pascal(entity.path())),
            path_snake(entity.path()),
        ] {
            reserve_name(&mut names, name)?;
        }
        if Fsm::try_from_entity(entity)
            .expect("schema was validated")
            .is_some()
        {
            for state in entity.events() {
                reserve_name(
                    &mut names,
                    format!(
                        "{}{}Handle",
                        path_pascal(entity.path()),
                        to_case(state.name(), Case::Pascal),
                    ),
                )?;
            }
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
        reserve_name(
            &mut names,
            format!("{}Input", name.trim_end_matches("Dict")),
        )?;
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
    for entity in public_entities(schema) {
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

fn helpers(options: &Options, runtime: &syn::Path, dynamic: &syn::Path) -> TokenStream {
    let module = raw_ident(
        options
            .module_name
            .rsplit('.')
            .next()
            .unwrap_or(&options.module_name),
    );
    quote! {
        use pyo3::prelude::*;
        use pyo3::types::{
            PyAny, PyBool, PyBoolMethods, PyFloat, PyFloatMethods, PyInt, PyMapping,
            PyMappingMethods, PyModule, PyString, PyStringMethods, PyTuple,
        };

        pyo3::create_exception!(#module, QuentError, pyo3::exceptions::PyException);
        pyo3::create_exception!(#module, EventAlreadyEmittedError, QuentError);
        pyo3::create_exception!(#module, SourceActivationError, QuentError);
        pyo3::create_exception!(#module, ContextClosedError, QuentError);
        pyo3::create_exception!(#module, HandleConsumedError, QuentError);

        fn __handle_error(error: #runtime::HandleError) -> PyErr {
            let message = error.to_string();
            match error {
                #runtime::HandleError::OnceAlreadyEmitted { .. } => {
                    EventAlreadyEmittedError::new_err(message)
                }
                #runtime::HandleError::SourceActivation { .. } => {
                    SourceActivationError::new_err(message)
                }
            }
        }

        fn __python_uuid(py: Python<'_>, value: #runtime::Uuid) -> PyResult<Py<PyAny>> {
            Ok(py.import("uuid")?
                .getattr("UUID")?
                .call1((value.to_string(),))?
                .unbind())
        }

        fn __extract_uuid(value: &Bound<'_, PyAny>) -> PyResult<#runtime::Uuid> {
            let uuid_type = value.py().import("uuid")?.getattr("UUID")?;
            if !value.is_instance(&uuid_type)? {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "expected uuid.UUID",
                ));
            }
            Ok(#runtime::Uuid::from_u128(value.getattr("int")?.extract()?))
        }

        #[pyclass(name = "DynamicValue", frozen, skip_from_py_object)]
        #[derive(Clone)]
        pub struct PyDynamicValue { inner: #dynamic::DynamicValue }

        #[pymethods]
        impl PyDynamicValue {
            pub fn __repr__(&self) -> String {
                format!("DynamicValue({:?})", self.inner)
            }
            #[staticmethod]
            pub fn u8(value: u8) -> Self {
                Self { inner: #dynamic::DynamicValue::U8(value) }
            }
            #[staticmethod]
            pub fn u16(value: u16) -> Self {
                Self { inner: #dynamic::DynamicValue::U16(value) }
            }
            #[staticmethod]
            pub fn u32(value: u32) -> Self {
                Self { inner: #dynamic::DynamicValue::U32(value) }
            }
            #[staticmethod]
            pub fn u64(value: u64) -> Self {
                Self { inner: #dynamic::DynamicValue::U64(value) }
            }
            #[staticmethod]
            pub fn i8(value: i8) -> Self {
                Self { inner: #dynamic::DynamicValue::I8(value) }
            }
            #[staticmethod]
            pub fn i16(value: i16) -> Self {
                Self { inner: #dynamic::DynamicValue::I16(value) }
            }
            #[staticmethod]
            pub fn i32(value: i32) -> Self {
                Self { inner: #dynamic::DynamicValue::I32(value) }
            }
            #[staticmethod]
            pub fn i64(value: i64) -> Self {
                Self { inner: #dynamic::DynamicValue::I64(value) }
            }
            #[staticmethod]
            pub fn f32(value: f32) -> Self {
                Self { inner: #dynamic::DynamicValue::F32(value) }
            }
            #[staticmethod]
            pub fn f64(value: f64) -> Self {
                Self { inner: #dynamic::DynamicValue::F64(value) }
            }
            #[staticmethod]
            pub fn string(value: String) -> Self {
                Self { inner: #dynamic::DynamicValue::String(value) }
            }
            #[staticmethod]
            pub fn structure(value: &Bound<'_, PyAny>) -> PyResult<Self> {
                Ok(Self {
                    inner: #dynamic::DynamicValue::Struct(__extract_dynamic_struct(value)?),
                })
            }
            #[staticmethod]
            pub fn u8_list(values: Vec<u8>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::U8(values)) }
            }
            #[staticmethod]
            pub fn u16_list(values: Vec<u16>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::U16(values)) }
            }
            #[staticmethod]
            pub fn u32_list(values: Vec<u32>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::U32(values)) }
            }
            #[staticmethod]
            pub fn u64_list(values: Vec<u64>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::U64(values)) }
            }
            #[staticmethod]
            pub fn i8_list(values: Vec<i8>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::I8(values)) }
            }
            #[staticmethod]
            pub fn i16_list(values: Vec<i16>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::I16(values)) }
            }
            #[staticmethod]
            pub fn i32_list(values: Vec<i32>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::I32(values)) }
            }
            #[staticmethod]
            pub fn i64_list(values: Vec<i64>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::I64(values)) }
            }
            #[staticmethod]
            pub fn f32_list(values: Vec<f32>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::F32(values)) }
            }
            #[staticmethod]
            pub fn f64_list(values: Vec<f64>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::F64(values)) }
            }
            #[staticmethod]
            pub fn string_list(values: Vec<String>) -> Self {
                Self { inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::String(values)) }
            }
            #[staticmethod]
            pub fn struct_list(values: &Bound<'_, PyAny>) -> PyResult<Self> {
                let mut output = Vec::new();
                for value in values.try_iter()? {
                    output.push(__extract_dynamic_struct(&value?)?);
                }
                Ok(Self {
                    inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::Struct(output)),
                })
            }
            #[staticmethod]
            pub fn list(values: &Bound<'_, PyAny>) -> PyResult<Self> {
                let mut output = Vec::new();
                for value in values.try_iter()? {
                    let value = value?.extract::<PyRef<'_, PyDynamicValue>>().map_err(|_| {
                        pyo3::exceptions::PyTypeError::new_err(
                            "nested dynamic lists require DynamicValue list elements",
                        )
                    })?;
                    let #dynamic::DynamicValue::List(value) = &value.inner else {
                        return Err(pyo3::exceptions::PyTypeError::new_err(
                            "nested dynamic lists require DynamicValue list elements",
                        ));
                    };
                    output.push(value.clone());
                }
                Ok(Self {
                    inner: #dynamic::DynamicValue::List(#dynamic::DynamicList::List(output)),
                })
            }
        }

        fn __extract_dynamic_struct(
            value: &Bound<'_, PyAny>,
        ) -> PyResult<#dynamic::DynamicStruct> {
            let values = value.cast::<PyMapping>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err("expected mapping for dynamic structure")
            })?;
            let mut output = Vec::new();
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
                let value = __extract_dynamic_value(&value).map_err(|error| {
                    pyo3::exceptions::PyTypeError::new_err(format!(
                        "invalid dynamic attribute value for `{key}`: {error}",
                    ))
                })?;
                output.push(#dynamic::DynamicAttribute { key, value });
            }
            Ok(#dynamic::DynamicStruct(output))
        }

        fn __extract_dynamic_value(
            value: &Bound<'_, PyAny>,
        ) -> PyResult<Option<#dynamic::DynamicValue>> {
            if value.is_none() {
                Ok(None)
            } else if let Ok(value) = value.extract::<PyRef<'_, PyDynamicValue>>() {
                Ok(Some(value.inner.clone()))
            } else if let Ok(value) = value.cast::<PyBool>() {
                Ok(Some(#dynamic::DynamicValue::U8(u8::from(value.is_true()))))
            } else if let Ok(value) = value.cast::<PyInt>() {
                if let Ok(value) = value.extract::<i64>() {
                    Ok(Some(#dynamic::DynamicValue::I64(value)))
                } else {
                    Ok(Some(#dynamic::DynamicValue::U64(value.extract::<u64>()?)))
                }
            } else if let Ok(value) = value.cast::<PyFloat>() {
                Ok(Some(#dynamic::DynamicValue::F64(value.value())))
            } else if let Ok(value) = value.cast::<PyString>() {
                Ok(Some(#dynamic::DynamicValue::String(value.to_str()?.to_owned())))
            } else if value.cast::<PyMapping>().is_ok() {
                Ok(Some(#dynamic::DynamicValue::Struct(
                    __extract_dynamic_struct(value)?,
                )))
            } else {
                Err(pyo3::exceptions::PyTypeError::new_err(
                    "expected None, bool, int, float, str, mapping, or DynamicValue",
                ))
            }
        }

        fn __extract_dynamic_attributes(
            value: &Bound<'_, PyAny>,
        ) -> PyResult<#runtime::DynamicAttributes> {
            Ok(#runtime::DynamicAttributes::from(
                __extract_dynamic_struct(value)?.0,
            ))
        }

        #[pyfunction]
        pub fn now_v7(py: Python<'_>) -> PyResult<Py<PyAny>> {
            __python_uuid(py, #runtime::Uuid::now_v7())
        }
        #[pyfunction]
        pub fn nil_uuid(py: Python<'_>) -> PyResult<Py<PyAny>> {
            __python_uuid(py, #runtime::Uuid::nil())
        }
    }
}

fn context(
    schema: &Schema,
    options: &Options,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
    io: &syn::Path,
) -> TokenStream {
    let has_nvtx_source = has_nvtx_source(schema);
    let model = model_path(instrumentation, schema.name());
    let context_ty = quote! { #instrumentation::Context<#model> };
    let module_name = &options.module_name;
    let observer_methods = public_entities(schema).map(|entity| {
        let method = raw_ident(py_safe(&format!("{}_observer", path_snake(entity.path()))));
        let observer = format_ident!("Py{}Observer", path_pascal(entity.path()));
        let entity_ty = rust_path(instrumentation, entity.path(), "");
        quote! {
            pub fn #method(&self) -> PyResult<#observer> {
                let context = self.inner.as_ref().ok_or_else(|| {
                    ContextClosedError::new_err(
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
        if has_nvtx_source {
            quote! {
                Some(ExporterKind::Options(options)) =>
                    <#context_ty>::try_new_with_options(options.clone(), context_options),
            }
        } else {
            quote! {
                Some(ExporterKind::Options(options)) => <#context_ty>::try_new(options.clone()),
            }
        }
    });
    let source_capture_type = has_nvtx_source.then(|| {
        quote! {
            /// Controls whether this context activates private live-capture sources.
            #[pyclass(name = "SourceCapture", eq, from_py_object)]
            #[derive(Clone, Copy, PartialEq, Eq)]
            pub enum PySourceCapture {
                Enabled,
                Disabled,
            }
        }
    });
    let constructor_signature = if has_nvtx_source {
        quote! {
            #[pyo3(signature = (options = None, *, source_capture = PySourceCapture::Enabled))]
        }
    } else {
        quote! { #[pyo3(signature = (options = None))] }
    };
    let source_capture_parameter =
        has_nvtx_source.then(|| quote! { , source_capture: PySourceCapture });
    let context_options = has_nvtx_source.then(|| {
        quote! {
            let source_capture = match source_capture {
                PySourceCapture::Enabled => #runtime::SourceCapture::Enabled,
                PySourceCapture::Disabled => #runtime::SourceCapture::Disabled,
            };
            let context_options = #runtime::ContextOptions::default()
                .with_source_capture(source_capture);
        }
    });
    let noop_context = if has_nvtx_source {
        quote! { <#context_ty>::try_new_with_options(#runtime::Noop, context_options) }
    } else {
        quote! { <#context_ty>::try_new(#runtime::Noop) }
    };
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
    let exporter_py_methods = (!exporter_methods.is_empty()).then(|| {
        quote! {
            #[pymethods]
            impl PyExporterOptions {
                #(#exporter_methods)*
            }
        }
    });
    quote! {
        #source_capture_type

        #[allow(dead_code)]
        enum ExporterKind { Noop, #option_variant }

        /// Configures the exporter used by a telemetry context.
        #[pyclass(name = "ExporterOptions", frozen)]
        pub struct PyExporterOptions { inner: ExporterKind }

        impl PyExporterOptions {
            #filesystem_helper
        }

        #exporter_py_methods

        /// Owns observer factories for one telemetry context.
        #[pyclass(name = "Context")]
        pub struct PyContext { inner: Option<#context_ty> }

        #[pymethods]
        impl PyContext {
            #[new]
            #constructor_signature
            pub fn new(
                options: Option<PyRef<'_, PyExporterOptions>>
                #source_capture_parameter
            ) -> PyResult<Self> {
                #context_options
                let result = match options.as_deref().map(|options| &options.inner) {
                    None | Some(ExporterKind::Noop) => #noop_context,
                    #option_match
                };
                let inner = result.map_err(|error| {
                    QuentError::new_err(error.to_string())
                })?;
                Ok(Self { inner: Some(inner) })
            }
            #[getter]
            pub fn id(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
                let context = self.inner.as_ref().ok_or_else(|| {
                    ContextClosedError::new_err(format!("`{}` context is closed", #module_name))
                })?;
                __python_uuid(py, context.id())
            }
            #[getter]
            pub fn closed(&self) -> bool { self.inner.is_none() }
            pub fn __repr__(&self) -> String {
                match self.inner.as_ref() {
                    Some(context) => format!("Context(id='{}', closed=False)", context.id()),
                    None => "Context(closed=True)".to_owned(),
                }
            }
            /// Prevents this context from creating additional observers.
            ///
            /// Existing observers and handles remain usable and retain the exporter.
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
    if let Some(fsm) = Fsm::try_from_entity(entity)
        .map_err(|error| GenerateError::InvalidSchema(error.to_string()))?
    {
        return fsm_entity_bindings(schema, entity, &fsm, instrumentation, runtime);
    }

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
            let signature = (!args.is_empty()).then(|| {
                quote! {
                    #[pyo3(signature = (*, #(#args),*))]
                }
            });
            let event_method = match event.cardinality() {
                Cardinality::Once => quote! {
                    #signature
                    #[allow(clippy::too_many_arguments)]
                    pub fn #method(&mut self, #(#params),*) -> PyResult<()> {
                        #(#bindings)*
                        self.inner.#model_method(#(#args),*)
                            .map_err(__handle_error)
                    }
                },
                Cardinality::Multi => quote! {
                    #signature
                    #[allow(clippy::too_many_arguments)]
                    pub fn #method(&self, #(#params),*) -> PyResult<()> {
                        #(#bindings)*
                        self.inner.#model_method(#(#args),*)
                            .map_err(|error| QuentError::new_err(error.to_string()))
                    }
                },
            };
            let emitted = (event.cardinality() == Cardinality::Once).then(|| {
                let emitted = raw_ident(py_safe(&format!(
                    "{}_emitted",
                    to_case(event.name(), Case::Snake)
                )));
                let model_emitted =
                    raw_ident(format!("{}_emitted", to_case(event.name(), Case::Snake)));
                quote! {
                    pub fn #emitted(&self) -> bool { self.inner.#model_emitted() }
                }
            });
            Ok::<_, GenerateError>(quote! { #event_method #emitted })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! {
        /// Creates handles for this entity type.
        #[pyclass(name = #observer_export)]
        pub struct #observer { inner: #instrumentation::Observer<#entity_ty> }

        #[pymethods]
        impl #observer {
            #[pyo3(signature = (id=None))]
            pub fn handle(&self, id: Option<&Bound<'_, PyAny>>) -> PyResult<#handle> {
                let inner = match id {
                    Some(id) => self.inner.handle_with_id(__extract_uuid(id)?),
                    None => self.inner.handle(),
                };
                Ok(#handle { inner })
            }
        }

        /// Emits events for one entity instance.
        #[pyclass(name = #handle_export)]
        pub struct #handle { inner: #instrumentation::Handle<#entity_ty> }

        impl #handle {
            fn raw_uuid(&self) -> PyResult<#runtime::Uuid> { Ok(self.inner.uuid()) }
        }

        #[pymethods]
        impl #handle {
            #[getter]
            pub fn uuid(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
                __python_uuid(py, self.raw_uuid()?)
            }
            pub fn __repr__(&self) -> String {
                format!("{}(uuid='{}')", #handle_export, self.inner.uuid())
            }
            #(#methods)*
        }
    })
}

fn fsm_entity_bindings(
    schema: &Schema,
    entity: &quent_schema::Entity,
    fsm: &Fsm,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
) -> Result<TokenStream, GenerateError> {
    let name = path_pascal(entity.path());
    let observer = format_ident!("Py{name}Observer");
    let initial_handle = format_ident!("Py{name}Handle");
    let observer_export = format!("{name}Observer");
    let initial_handle_export = format!("{name}Handle");
    let entity_ty = rust_path(instrumentation, entity.path(), "");
    let modules = entity
        .path()
        .namespace()
        .iter()
        .map(|part| raw_ident(to_case(part, Case::Snake)));
    let state_module = raw_ident(format!(
        "{}_state",
        to_case(entity.path().name(), Case::Snake)
    ));
    let state_module = quote! { #instrumentation::#(#modules::)*#state_module };
    let initial_event = entity
        .event(fsm.initial_state())
        .expect("validated FSM initial state");
    let initial_target = fsm_state_handle_ident(entity, initial_event.name());
    let initial_method = fsm_transition_method(
        schema,
        initial_event,
        &initial_target,
        instrumentation,
        runtime,
    )?;

    let state_classes = entity
        .events()
        .map(|state| {
            let handle = fsm_state_handle_ident(entity, state.name());
            let handle_export = format!(
                "{}{}Handle",
                path_pascal(entity.path()),
                to_case(state.name(), Case::Pascal),
            );
            let marker = raw_ident(to_case(state.name(), Case::Pascal));
            let methods = fsm
                .transitions()
                .iter()
                .filter(|transition| transition.source() == state.name())
                .map(|transition| {
                    let target_event = entity
                        .event(transition.target())
                        .expect("validated FSM transition target");
                    let target = fsm_state_handle_ident(entity, target_event.name());
                    fsm_transition_method(schema, target_event, &target, instrumentation, runtime)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let state_name = state.name().to_string();
            Ok::<_, GenerateError>(quote! {
                /// Represents one FSM entity in this state.
                #[pyclass(name = #handle_export)]
                pub struct #handle {
                    inner: Option<#instrumentation::FsmHandle<#entity_ty, #state_module::#marker>>,
                }

                impl #handle {
                    fn raw_uuid(&self) -> PyResult<#runtime::Uuid> {
                        self.inner
                            .as_ref()
                            .map(|inner| inner.uuid())
                            .ok_or_else(|| HandleConsumedError::new_err("FSM handle was consumed"))
                    }
                }

                #[pymethods]
                impl #handle {
                    #[getter]
                    pub fn uuid(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
                        __python_uuid(py, self.raw_uuid()?)
                    }
                    pub fn __repr__(&self) -> String {
                        match self.inner.as_ref() {
                            Some(inner) => format!(
                                "{}(uuid='{}', state='{}')",
                                #handle_export,
                                inner.uuid(),
                                #state_name,
                            ),
                            None => format!("{}(consumed=True)", #handle_export),
                        }
                    }
                    #(#methods)*
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(quote! {
        /// Creates handles for this FSM entity type.
        #[pyclass(name = #observer_export)]
        pub struct #observer { inner: #instrumentation::Observer<#entity_ty> }

        #[pymethods]
        impl #observer {
            #[pyo3(signature = (id=None))]
            pub fn handle(&self, id: Option<&Bound<'_, PyAny>>) -> PyResult<#initial_handle> {
                let inner = match id {
                    Some(id) => self.inner.handle_with_id(__extract_uuid(id)?),
                    None => self.inner.handle(),
                };
                Ok(#initial_handle { inner: Some(inner) })
            }
        }

        /// Starts one FSM entity instance.
        #[pyclass(name = #initial_handle_export)]
        pub struct #initial_handle {
            inner: Option<#instrumentation::FsmHandle<#entity_ty>>,
        }

        impl #initial_handle {
            fn raw_uuid(&self) -> PyResult<#runtime::Uuid> {
                self.inner
                    .as_ref()
                    .map(|inner| inner.uuid())
                    .ok_or_else(|| HandleConsumedError::new_err("FSM handle was consumed"))
            }
        }

        #[pymethods]
        impl #initial_handle {
            #[getter]
            pub fn uuid(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
                __python_uuid(py, self.raw_uuid()?)
            }
            pub fn __repr__(&self) -> String {
                match self.inner.as_ref() {
                    Some(inner) => format!("{}(uuid='{}')", #initial_handle_export, inner.uuid()),
                    None => format!("{}(consumed=True)", #initial_handle_export),
                }
            }
            #initial_method
        }

        #(#state_classes)*
    })
}

fn fsm_state_handle_ident(
    entity: &quent_schema::Entity,
    state: &quent_schema::Identifier,
) -> syn::Ident {
    format_ident!(
        "Py{}{}Handle",
        path_pascal(entity.path()),
        to_case(state, Case::Pascal),
    )
}

fn fsm_transition_method(
    schema: &Schema,
    event: &quent_schema::Event,
    target_handle: &syn::Ident,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
) -> Result<TokenStream, GenerateError> {
    let model_method = raw_ident(to_case(event.name(), Case::Snake));
    let method = raw_ident(py_safe(&to_case(event.name(), Case::Snake)));
    let fields = event
        .fields()
        .filter(|field| field.name() != SEQUENCE_FIELD_NAME)
        .collect::<Vec<_>>();
    let params = fields.iter().map(|field| {
        let name = raw_ident(py_safe(&to_case(field.name(), Case::Snake)));
        quote! { #name: &Bound<'_, PyAny> }
    });
    let bindings = fields
        .iter()
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
    let args = fields
        .iter()
        .map(|field| raw_ident(py_safe(&to_case(field.name(), Case::Snake))))
        .collect::<Vec<_>>();
    let signature = (!args.is_empty()).then(|| {
        quote! {
            #[pyo3(signature = (*, #(#args),*))]
        }
    });
    Ok(quote! {
        #signature
        #[allow(clippy::too_many_arguments)]
        pub fn #method(&mut self, #(#params),*) -> PyResult<#target_handle> {
            #(#bindings)*
            let inner = self.inner.take().ok_or_else(|| {
                HandleConsumedError::new_err("FSM handle was consumed")
            })?;
            Ok(#target_handle { inner: Some(inner.#model_method(#(#args),*)) })
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
    let observers = public_entities(schema)
        .map(|entity| format_ident!("Py{}Observer", path_pascal(entity.path())));
    let handles = public_entities(schema).flat_map(|entity| {
        let mut handles = vec![format_ident!("Py{}Handle", path_pascal(entity.path()))];
        if Fsm::try_from_entity(entity)
            .expect("schema was validated")
            .is_some()
        {
            handles.extend(
                entity
                    .events()
                    .map(|state| fsm_state_handle_ident(entity, state.name())),
            );
        }
        handles
    });
    let source_capture = has_nvtx_source(schema).then(|| {
        quote! { module.add_class::<PySourceCapture>()?; }
    });
    quote! {
        #[pymodule(name = #export_name)]
        pub fn #rust_name(module: &Bound<'_, PyModule>) -> PyResult<()> {
            module.add("QuentError", module.py().get_type::<QuentError>())?;
            module.add(
                "EventAlreadyEmittedError",
                module.py().get_type::<EventAlreadyEmittedError>(),
            )?;
            module.add(
                "SourceActivationError",
                module.py().get_type::<SourceActivationError>(),
            )?;
            module.add(
                "ContextClosedError",
                module.py().get_type::<ContextClosedError>(),
            )?;
            module.add(
                "HandleConsumedError",
                module.py().get_type::<HandleConsumedError>(),
            )?;
            module.add_function(wrap_pyfunction!(now_v7, module)?)?;
            module.add_function(wrap_pyfunction!(nil_uuid, module)?)?;
            module.add_class::<PyDynamicValue>()?;
            module.add_class::<PyExporterOptions>()?;
            #source_capture
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
