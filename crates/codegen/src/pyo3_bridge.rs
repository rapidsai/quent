// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! PyO3 bridge code generator.
//!
//! Generates a Rust extension-module wrapper from model definitions. The
//! generated code is ordinary Rust using `#[pyclass]`, `#[pymethods]`,
//! `#[pyfunction]`, and `#[pymodule]`, so it can be included at the root of a
//! `cdylib` crate compiled by PyO3/maturin.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

use quent_model::{AttributeDef, FsmDef, ModelBuilder, StateDef, UsageDef, ValueType};

use crate::common::{
    is_auto_declaration_event, pretty_print, quent_path, remap_module_path,
    resource_operating_attrs, to_pascal_case,
};
use crate::{GeneratedFile, PyO3Options};

/// Convert a model-provided Rust identifier into the Python identifier spelling
/// exported by generated bindings.
///
/// The model definition language supplies ordinary Rust identifiers, so the
/// only Python-specific conflict we handle here is a Python reserved word.
pub(crate) fn py_export_name(name: &str) -> String {
    let mut out = name.to_string();
    if matches!(
        name,
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
    ) {
        println!(
            "cargo:warning=model component `{name}` is a Python reserved keyword. \
             The exposed Python name will be `{name}_`"
        );
        out.push('_');
    }
    out
}

/// Turn a dotted Python module name into the local Rust identifier used for the
/// `#[pymodule]` function.
fn module_ident(name: &str) -> syn::Ident {
    format_ident!("{}", name.replace('.', "_"))
}

fn module_export_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

fn py_class_ident(name: &str) -> syn::Ident {
    format_ident!("Py{}", py_export_name(name))
}

fn event_enum_ident(component_name: &str) -> syn::Ident {
    format_ident!("{}Event", to_pascal_case(component_name))
}

fn type_from_model_path(type_path: &str, component_mod: &syn::Path) -> syn::Type {
    if type_path.contains("::") {
        syn::parse_str(type_path).unwrap()
    } else {
        syn::parse_str(&format!("{}::{}", quote!(#component_mod), type_path)).unwrap()
    }
}

fn value_type_rust_extract(ty: &ValueType) -> TokenStream {
    match ty {
        ValueType::Bool => quote! { bool },
        ValueType::U8 => quote! { u8 },
        ValueType::U16 => quote! { u16 },
        ValueType::U32 => quote! { u32 },
        ValueType::U64 => quote! { u64 },
        ValueType::I8 => quote! { i8 },
        ValueType::I16 => quote! { i16 },
        ValueType::I32 => quote! { i32 },
        ValueType::I64 => quote! { i64 },
        ValueType::F32 => quote! { f32 },
        ValueType::F64 => quote! { f64 },
        ValueType::String => quote! { String },
        ValueType::Uuid
        | ValueType::Ref(_)
        | ValueType::CustomAttributes
        | ValueType::List(_)
        | ValueType::Struct(_, _) => quote! {},
    }
}

fn emit_pyany_conversion_expr(
    ty: &ValueType,
    optional: bool,
    obj: TokenStream,
    q: &syn::Path,
    component_mod: &syn::Path,
) -> TokenStream {
    if optional {
        let inner = emit_pyany_conversion_expr(ty, false, obj.clone(), q, component_mod);
        return quote! {
            if (#obj).is_none() {
                None
            } else {
                Some(#inner)
            }
        };
    }

    match ty {
        ValueType::Bool
        | ValueType::U8
        | ValueType::U16
        | ValueType::U32
        | ValueType::U64
        | ValueType::I8
        | ValueType::I16
        | ValueType::I32
        | ValueType::I64
        | ValueType::F32
        | ValueType::F64
        | ValueType::String => {
            let extract_ty = value_type_rust_extract(ty);
            quote! { (#obj).extract::<#extract_ty>()? }
        }
        ValueType::Uuid => quote! { __extract_uuid(#obj)? },
        ValueType::Ref(_) => quote! { #q::Ref::new(__extract_uuid(#obj)?) },
        ValueType::CustomAttributes => quote! { __extract_custom_attributes(#obj)? },
        ValueType::List(inner) => emit_pyany_list_conversion_expr(inner, obj, q, component_mod),
        ValueType::Struct(type_path, attrs) => {
            emit_pyany_struct_conversion_expr(type_path, attrs, obj, q, component_mod)
        }
    }
}

fn emit_attr_param_and_binding(
    name: &syn::Ident,
    ty: &ValueType,
    optional: bool,
    q: &syn::Path,
    component_mod: &syn::Path,
) -> (TokenStream, TokenStream) {
    match (ty, optional) {
        (ValueType::Uuid, false) => (
            quote! { #name: PyRef<'_, PyUuid> },
            quote! {
                let #name = #name.inner;
            },
        ),
        (ValueType::Uuid, true) => (
            quote! { #name: Option<PyRef<'_, PyUuid>> },
            quote! {
                let #name = #name.map(|value| value.inner);
            },
        ),
        (ValueType::Ref(_), false) => (
            quote! { #name: PyRef<'_, PyUuid> },
            quote! {
                let #name = #q::Ref::new(#name.inner);
            },
        ),
        (ValueType::Ref(_), true) => (
            quote! { #name: Option<PyRef<'_, PyUuid>> },
            quote! {
                let #name = #name.map(|value| #q::Ref::new(value.inner));
            },
        ),
        _ => {
            let conv = emit_pyany_conversion_expr(ty, optional, quote! { #name }, q, component_mod);
            (
                quote! { #name: &Bound<'_, PyAny> },
                quote! {
                    let #name = #conv;
                },
            )
        }
    }
}

fn emit_pyany_list_conversion_expr(
    inner: &ValueType,
    obj: TokenStream,
    q: &syn::Path,
    component_mod: &syn::Path,
) -> TokenStream {
    let inner_expr = emit_pyany_conversion_expr(inner, false, quote! { &item }, q, component_mod);
    quote! {
        {
            let list = (#obj).cast::<pyo3::types::PyList>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err("expected a Python list")
            })?;
            let mut out = Vec::with_capacity(list.len());
            for item in list.iter() {
                out.push(#inner_expr);
            }
            out
        }
    }
}

fn emit_pyany_struct_conversion_expr(
    type_path: &str,
    attrs: &[AttributeDef],
    obj: TokenStream,
    q: &syn::Path,
    component_mod: &syn::Path,
) -> TokenStream {
    let model_ty = type_from_model_path(type_path, component_mod);
    let fields: Vec<TokenStream> = attrs
        .iter()
        .map(|attr| {
            let field = format_ident!("{}", attr.name);
            let field_name = py_export_name(&attr.name);
            if attr.optional {
                let conv = emit_pyany_conversion_expr(
                    &attr.value_type,
                    true,
                    quote! { &value },
                    q,
                    component_mod,
                );
                quote! {
                    #field: match dict.get_item(#field_name)? {
                        Some(value) => #conv,
                        None => None,
                    }
                }
            } else {
                let conv = emit_pyany_conversion_expr(
                    &attr.value_type,
                    false,
                    quote! { &value },
                    q,
                    component_mod,
                );
                quote! {
                    #field: {
                        let value = dict.get_item(#field_name)?.ok_or_else(|| {
                            pyo3::exceptions::PyKeyError::new_err(
                                format!("missing required field `{}`", #field_name)
                            )
                        })?;
                        #conv
                    }
                }
            }
        })
        .collect();

    quote! {
        {
            let dict = (#obj).cast::<PyDict>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err(
                    format!("expected dict for `{}`", #type_path)
                )
            })?;
            #model_ty {
                #(#fields,)*
            }
        }
    }
}

fn usage_resource_type(usage: &UsageDef, component_mod_str: &str) -> syn::Type {
    if usage.resource_type_path.contains("::") {
        syn::parse_str(&usage.resource_type_path).unwrap()
    } else {
        syn::parse_str(&format!(
            "{}::{}",
            component_mod_str, usage.resource_type_path
        ))
        .unwrap()
    }
}

fn emit_usage_conversion_expr(
    usage: &UsageDef,
    capacity_attrs: &[AttributeDef],
    obj: TokenStream,
    q: &syn::Path,
    component_mod_str: &str,
    component_mod: &syn::Path,
) -> TokenStream {
    let resource_ty = usage_resource_type(usage, component_mod_str);
    let resource_handle =
        py_class_ident(&format!("{}Handle", to_pascal_case(&usage.resource_name)));
    let resource_handle_name = format!("{}Handle", to_pascal_case(&usage.resource_name));

    if capacity_attrs.is_empty() {
        quote! {
            if (#obj).is_none() {
                None
            } else {
                let resource = (#obj).extract::<PyRef<'_, #resource_handle>>().map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err(
                        format!("expected {} usage argument", #resource_handle_name)
                    )
                })?;
                let resource_id = resource.inner.uuid();
                Some(#q::Usage {
                    resource_id: #q::Ref::new(resource_id),
                    capacity: <#resource_ty as #q::Resource>::CapacityValue::default(),
                })
            }
        }
    } else {
        let cap_values: Vec<TokenStream> = capacity_attrs
            .iter()
            .enumerate()
            .map(|(idx, attr)| {
                let arg_idx = idx + 1;
                let cap_name = &attr.name;
                let value = quote! { &value };
                let conv =
                    emit_pyany_conversion_expr(&attr.value_type, false, value, q, component_mod);
                quote! {
                    {
                        let value = __usage_arg_item(#obj, #arg_idx)?.ok_or_else(|| {
                            pyo3::exceptions::PyValueError::new_err(
                                format!("usage argument is missing capacity `{}`", #cap_name)
                            )
                        })?;
                        #conv
                    }
                }
            })
            .collect();

        quote! {
            if (#obj).is_none() {
                None
            } else {
                let resource_obj = __usage_arg_item(#obj, 0)?.ok_or_else(|| {
                    pyo3::exceptions::PyValueError::new_err("usage argument is missing a resource handle")
                })?;
                let resource = resource_obj.extract::<PyRef<'_, #resource_handle>>().map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err(
                        format!("expected {} usage argument", #resource_handle_name)
                    )
                })?;
                let resource_id = resource.inner.uuid();
                let has_capacity_values = __usage_arg_item(#obj, 1)?.is_some();
                let capacity = if has_capacity_values {
                    <#resource_ty as #q::Resource>::CapacityValue::from((#(#cap_values,)*))
                } else {
                    <#resource_ty as #q::Resource>::CapacityValue::default()
                };
                Some(#q::Usage {
                    resource_id: #q::Ref::new(resource_id),
                    capacity,
                })
            }
        }
    }
}

fn emit_state_args(
    model: &ModelBuilder,
    state: &StateDef,
    q: &syn::Path,
    component_mod_str: &str,
    component_mod: &syn::Path,
) -> (Vec<TokenStream>, TokenStream, Vec<TokenStream>) {
    let mut params = Vec::new();
    let mut bindings = Vec::new();
    let mut args = Vec::new();

    for attr in &state.attributes {
        let name = format_ident!("{}", py_export_name(&attr.name));
        let (param, binding) =
            emit_attr_param_and_binding(&name, &attr.value_type, attr.optional, q, component_mod);
        params.push(param);
        bindings.push(binding);
        if attr.name == "instance_name" && attr.value_type == ValueType::String && !attr.optional {
            args.push(quote! { #name.as_str() });
        } else {
            args.push(quote! { #name });
        }
    }

    for usage in &state.usages {
        let name = format_ident!("{}", py_export_name(&usage.field_name));
        params.push(quote! { #name: &Bound<'_, PyAny> });
        let capacity_attrs = resource_operating_attrs(model, usage);
        let conv = emit_usage_conversion_expr(
            usage,
            &capacity_attrs,
            quote! { #name },
            q,
            component_mod_str,
            component_mod,
        );
        bindings.push(quote! {
            let #name = #conv;
        });
        args.push(quote! { #name });
    }

    (params, quote! { #(#bindings)* }, args)
}

fn emit_helpers(q: &syn::Path) -> TokenStream {
    quote! {
        use pyo3::prelude::*;
        use pyo3::types::{
            PyAny, PyBool, PyBoolMethods, PyDict, PyFloat, PyFloatMethods, PyInt, PyModule,
            PyString, PyStringMethods, PyTuple,
        };

        #[pyclass(name = "Uuid", frozen, skip_from_py_object)]
        #[derive(Clone)]
        pub struct PyUuid {
            inner: #q::uuid::Uuid,
        }

        #[pymethods]
        impl PyUuid {
            pub fn __repr__(&self) -> String {
                format!("Uuid('{}')", self.inner)
            }

            pub fn __str__(&self) -> String {
                self.inner.to_string()
            }

            pub fn __richcmp__(
                &self,
                other: &Bound<'_, PyAny>,
                op: pyo3::class::basic::CompareOp,
            ) -> PyResult<bool> {
                match op {
                    pyo3::class::basic::CompareOp::Eq => {
                        let Ok(other) = other.extract::<PyRef<'_, PyUuid>>() else {
                            return Ok(false);
                        };
                        Ok(self.inner == other.inner)
                    }
                    pyo3::class::basic::CompareOp::Ne => {
                        let Ok(other) = other.extract::<PyRef<'_, PyUuid>>() else {
                            return Ok(true);
                        };
                        Ok(self.inner != other.inner)
                    }
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

        fn __extract_uuid(obj: &Bound<'_, PyAny>) -> PyResult<#q::uuid::Uuid> {
            Ok(obj.extract::<PyRef<'_, PyUuid>>()
                .map(|value| value.inner)?)
        }

        fn __usage_arg_item<'py>(
            obj: &Bound<'py, PyAny>,
            index: usize,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            if obj.is_none() {
                return Ok(None);
            }
            if let Ok(tuple) = obj.cast::<PyTuple>() {
                if index < tuple.len() {
                    return Ok(Some(tuple.get_item(index)?));
                }
                return Ok(None);
            }
            if index == 0 {
                Ok(Some(obj.clone()))
            } else {
                Ok(None)
            }
        }

        fn __extract_custom_attributes(
            obj: &Bound<'_, PyAny>,
        ) -> PyResult<#q::attributes::CustomAttributes> {
            let dict = obj.cast::<PyDict>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err(
                    "expected dict for custom attributes",
                )
            })?;
            let mut attrs = #q::attributes::CustomAttributes::new();
            for (key, value) in dict.iter() {
                let key = key
                    .cast::<PyString>()
                    .map_err(|_| {
                        pyo3::exceptions::PyTypeError::new_err(
                            "custom attribute keys must be strings",
                        )
                    })?
                    .to_str()?
                    .to_owned();
                if value.is_none() {
                    attrs.add(#q::attributes::Attribute::null(key));
                } else if let Ok(value) = value.cast::<PyBool>() {
                    attrs.add_bool(key, value.is_true());
                } else if let Ok(value) = value.cast::<PyInt>() {
                    attrs.add_i64(key, value.extract::<i64>()?);
                } else if let Ok(value) = value.cast::<PyFloat>() {
                    attrs.add_f64(key, value.value());
                } else if let Ok(value) = value.cast::<PyString>() {
                    attrs.add_string(key, value.to_str()?);
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                        "unsupported custom attribute value for `{key}`"
                    )));
                }
            }
            Ok(attrs)
        }

        #[pyfunction]
        pub fn now_v7() -> PyUuid {
            PyUuid {
                inner: #q::uuid::Uuid::now_v7(),
            }
        }

        #[pyfunction]
        pub fn nil_uuid() -> PyUuid {
            PyUuid {
                inner: #q::uuid::Uuid::nil(),
            }
        }
    }
}

fn emit_context(
    model: &ModelBuilder,
    q: &syn::Path,
    event_type: &syn::Type,
    options: &PyO3Options,
) -> TokenStream {
    let observer_methods: Vec<TokenStream> = model
        .entities
        .iter()
        .map(|entity| {
            let method = format_ident!("{}", py_export_name(&format!("{}_observer", entity.name)));
            let class = py_class_ident(&py_export_name(&format!(
                "{}Observer",
                to_pascal_case(&entity.name)
            )));
            quote! {
                pub fn #method(&self) -> PyResult<#class> {
                    Ok(#class {
                        tx: self.events_sender()?,
                    })
                }
            }
        })
        .chain(model.fsms.iter().map(|fsm| {
            let method = format_ident!("{}", py_export_name(&format!("{}_observer", fsm.name)));
            let class = py_class_ident(&py_export_name(&format!(
                "{}Observer",
                to_pascal_case(&fsm.name)
            )));
            quote! {
                pub fn #method(&self) -> PyResult<#class> {
                    Ok(#class {
                        tx: self.events_sender()?,
                    })
                }
            }
        }))
        .collect();

    let module_name = &options.module_name;

    quote! {
        #[pyclass(name = "Context")]
        pub struct PyContext {
            inner: Option<#q::Context<#event_type>>,
            tx: #q::EventSender<#event_type>,
            id: #q::uuid::Uuid,
        }

        #[pymethods]
        impl PyContext {
            #[new]
            pub fn new(
                exporter: Option<String>,
                output_dir: Option<String>,
            ) -> PyResult<Self> {
                let opts = match exporter.as_deref() {
                    Some("ndjson") => Some(#q::exporter::ExporterOptions::FileSystem(
                        #q::exporter::FileSystemExporterOptions {
                            format: #q::exporter::FileSystemFormat::Ndjson,
                            root: std::path::PathBuf::from(
                                output_dir.unwrap_or_else(|| ".".to_string()),
                            ),
                        },
                    )),
                    None => None,
                    Some(other) => {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "unsupported exporter `{other}` for generated PyO3 bridge; supported values: `'ndjson'`, `None`"
                        )));
                    }
                };
                let inner = #q::Context::try_new(opts)
                    .map_err(|err| pyo3::exceptions::PyRuntimeError::new_err(err.to_string()))?;
                let id = inner.id();
                let tx = inner.events_sender();
                Ok(Self {
                    inner: Some(inner),
                    tx,
                    id,
                })
            }

            #[getter]
            pub fn id(&self) -> PyUuid {
                PyUuid { inner: self.id }
            }

            pub fn close(&mut self) {
                self.inner.take();
            }

            pub fn __enter__(slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
                slf
            }

            pub fn __exit__(
                &mut self,
                _exc_type: &Bound<'_, PyAny>,
                _exc_value: &Bound<'_, PyAny>,
                _traceback: &Bound<'_, PyAny>,
            ) {
                self.close();
            }

            #(#observer_methods)*
        }

        impl PyContext {
            fn events_sender(&self) -> PyResult<#q::EventSender<#event_type>> {
                if self.inner.is_none() {
                    return Err(pyo3::exceptions::PyRuntimeError::new_err(
                        format!("`{}` context is closed", #module_name),
                    ));
                }
                Ok(self.tx.clone())
            }
        }
    }
}

fn emit_entity_bridge(
    entity: &quent_model::EntityDef,
    q: &syn::Path,
    event_type: &syn::Type,
    options: &PyO3Options,
) -> TokenStream {
    let multi_event = entity.events.len() > 1;
    let pascal_name = to_pascal_case(&entity.name);
    let observer_py_name = py_export_name(&format!("{pascal_name}Observer"));
    let handle_py_name = py_export_name(&format!("{pascal_name}Handle"));
    let observer = py_class_ident(&observer_py_name);
    let handle = py_class_ident(&handle_py_name);
    let component_mod: syn::Path =
        syn::parse_str(&remap_module_path(&entity.module_path, options)).unwrap();
    let entity_event_enum = event_enum_ident(&entity.name);

    let methods: Vec<TokenStream> = entity
        .events
        .iter()
        .map(|event| {
            let is_declaration = is_auto_declaration_event(&entity.name, &event.name);
            let method = if is_declaration {
                format_ident!("{}", py_export_name(&entity.name))
            } else {
                format_ident!("{}", py_export_name(&event.name))
            };
            let event_pascal = format_ident!("{}", to_pascal_case(&event.name));
            let mut params = if multi_event {
                Vec::new()
            } else {
                vec![quote! { id: PyRef<'_, PyUuid> }]
            };
            let mut bindings = if multi_event {
                Vec::new()
            } else {
                vec![quote! { let id = id.inner; }]
            };
            let mut field_inits = Vec::new();

            for attr in &event.attributes {
                let field = format_ident!("{}", py_export_name(&attr.name));
                let model_field = format_ident!("{}", attr.name);
                let (param, binding) = emit_attr_param_and_binding(
                    &field,
                    &attr.value_type,
                    attr.optional,
                    q,
                    &component_mod,
                );
                params.push(param);
                bindings.push(binding);
                if field == model_field {
                    field_inits.push(quote! { #field });
                } else {
                    field_inits.push(quote! { #model_field: #field });
                }
            }

            let model_event = if event.attributes.is_empty() {
                quote! { #component_mod::#event_pascal }
            } else {
                quote! {
                    #component_mod::#event_pascal {
                        #(#field_inits,)*
                    }
                }
            };

            let ret_ty = if is_declaration {
                quote! { PyUuid }
            } else {
                quote! { () }
            };
            let ret_expr = if is_declaration {
                if multi_event {
                    quote! { PyUuid { inner: self.id } }
                } else {
                    quote! { PyUuid { inner: id } }
                }
            } else {
                quote! { () }
            };

            let id_expr = if multi_event {
                quote! { self.id }
            } else {
                quote! { id }
            };

            quote! {
                pub fn #method(&self, #(#params,)*) -> PyResult<#ret_ty> {
                    #(#bindings)*
                    let model_event = #model_event;
                    self.tx.send(#q::Event::new_now(
                        #id_expr,
                        #component_mod::#entity_event_enum::from(model_event).into(),
                    ));
                    Ok(#ret_expr)
                }
            }
        })
        .collect();

    if multi_event {
        quote! {
            #[pyclass(name = #observer_py_name)]
            pub struct #observer {
                tx: #q::EventSender<#event_type>,
            }

            #[pymethods]
            impl #observer {
                pub fn create(&self, id: PyRef<'_, PyUuid>) -> PyResult<#handle> {
                    Ok(#handle {
                        id: id.inner,
                        tx: self.tx.clone(),
                    })
                }
            }

            #[pyclass(name = #handle_py_name)]
            pub struct #handle {
                id: #q::uuid::Uuid,
                tx: #q::EventSender<#event_type>,
            }

            #[pymethods]
            impl #handle {
                #[getter]
                pub fn uuid(&self) -> PyUuid {
                    PyUuid { inner: self.id }
                }

                #(#methods)*
            }
        }
    } else {
        quote! {
        #[pyclass(name = #observer_py_name)]
        pub struct #observer {
            tx: #q::EventSender<#event_type>,
        }

        #[pymethods]
        impl #observer {
            #(#methods)*
        }
        }
    }
}

fn emit_fsm_bridge(
    model: &ModelBuilder,
    fsm: &FsmDef,
    q: &syn::Path,
    event_type: &syn::Type,
    options: &PyO3Options,
) -> TokenStream {
    let pascal_name = to_pascal_case(&fsm.name);
    let observer_py_name = py_export_name(&format!("{pascal_name}Observer"));
    let handle_py_name = py_export_name(&format!("{pascal_name}Handle"));
    let observer = py_class_ident(&observer_py_name);
    let handle = py_class_ident(&handle_py_name);
    let component_mod_str = remap_module_path(&fsm.module_path, options);
    let component_mod: syn::Path = syn::parse_str(&component_mod_str).unwrap();
    let model_handle: syn::Type = syn::parse_str(&format!(
        "{}::{}Handle<{}>",
        component_mod_str,
        pascal_name,
        quote!(#event_type)
    ))
    .unwrap();

    let entry_state = fsm
        .states
        .iter()
        .find(|state| state.name == fsm.entry)
        .unwrap_or_else(|| {
            panic!(
                "entry state `{}` not found in FSM `{}`",
                fsm.entry, fsm.name
            )
        });
    let entry_method = format_ident!("{}", py_export_name(&entry_state.name));
    let model_entry_method = format_ident!("{}", entry_state.name);
    let observer_name = format_ident!("{}Observer", pascal_name);
    let (entry_params, entry_bindings, entry_args) =
        emit_state_args(model, entry_state, q, &component_mod_str, &component_mod);

    let transition_methods: Vec<TokenStream> = fsm
        .states
        .iter()
        .filter(|state| state.name != fsm.entry)
        .map(|state| {
            let method = format_ident!("{}", py_export_name(&state.name));
            let model_method = format_ident!("{}", state.name);
            if state.attributes.is_empty() && state.usages.is_empty() {
                quote! {
                    pub fn #method(&mut self) {
                        self.inner.#model_method();
                    }
                }
            } else {
                let (params, bindings, args) =
                    emit_state_args(model, state, q, &component_mod_str, &component_mod);
                quote! {
                    pub fn #method(&mut self, #(#params,)*) -> PyResult<()> {
                        #bindings
                        self.inner.#model_method(#(#args),*);
                        Ok(())
                    }
                }
            }
        })
        .collect();

    quote! {
        #[pyclass(name = #observer_py_name)]
        pub struct #observer {
            tx: #q::EventSender<#event_type>,
        }

        #[pymethods]
        impl #observer {
            pub fn #entry_method(
                &self,
                id: PyRef<'_, PyUuid>,
                #(#entry_params,)*
            ) -> PyResult<#handle> {
                let id = id.inner;
                #entry_bindings
                let obs = #component_mod::#observer_name::new(&self.tx);
                Ok(#handle {
                    inner: obs.#model_entry_method(id, #(#entry_args),*),
                })
            }
        }

        #[pyclass(name = #handle_py_name)]
        pub struct #handle {
            inner: #model_handle,
        }

        #[pymethods]
        impl #handle {
            #[getter]
            pub fn uuid(&self) -> PyUuid {
                PyUuid {
                    inner: self.inner.uuid(),
                }
            }

            #(#transition_methods)*

            pub fn exit(&mut self) {
                self.inner.exit();
            }
        }
    }
}

/// Generate PyO3 bridge source code from a model.
pub fn emit(model: &ModelBuilder, options: &PyO3Options) -> Vec<GeneratedFile> {
    let q = quent_path(&model.name, options);
    let event_type: syn::Type = syn::parse_str(&options.event_type(&model.name)).unwrap();
    let module_fn = module_ident(&options.module_name);
    let module_name = syn::LitStr::new(module_export_name(&options.module_name), Span::call_site());

    let helpers = emit_helpers(&q);
    let context = emit_context(model, &q, &event_type, options);
    let entity_bridges: Vec<TokenStream> = model
        .entities
        .iter()
        .map(|entity| emit_entity_bridge(entity, &q, &event_type, options))
        .collect();
    let fsm_bridges: Vec<TokenStream> = model
        .fsms
        .iter()
        .map(|fsm| emit_fsm_bridge(model, fsm, &q, &event_type, options))
        .collect();

    let observer_classes: Vec<syn::Ident> = model
        .entities
        .iter()
        .map(|entity| py_class_ident(&format!("{}Observer", to_pascal_case(&entity.name))))
        .chain(
            model
                .fsms
                .iter()
                .map(|fsm| py_class_ident(&format!("{}Observer", to_pascal_case(&fsm.name)))),
        )
        .collect();
    let handle_classes: Vec<syn::Ident> = model
        .fsms
        .iter()
        .map(|fsm| py_class_ident(&format!("{}Handle", to_pascal_case(&fsm.name))))
        .chain(
            model
                .entities
                .iter()
                .filter(|entity| entity.events.len() > 1)
                .map(|entity| py_class_ident(&format!("{}Handle", to_pascal_case(&entity.name)))),
        )
        .collect();
    let tokens = quote! {
        // Some of the emitted code borrows an already-borrowed value. In
        // theory we can track when a value is borrowed, but it makes the
        // codegen more complicated and the additional borrow is harmless.
        #[allow(clippy::needless_borrow)]
        mod __quent_pyo3_bridge {
            #helpers
            #context
            #(#entity_bridges)*
            #(#fsm_bridges)*

            #[pymodule(name = #module_name)]
            pub fn #module_fn(m: &Bound<'_, PyModule>) -> PyResult<()> {
                m.add_function(wrap_pyfunction!(now_v7, m)?)?;
                m.add_function(wrap_pyfunction!(nil_uuid, m)?)?;
                m.add_class::<PyUuid>()?;
                m.add_class::<PyContext>()?;
                #(
                    m.add_class::<#observer_classes>()?;
                )*
                #(
                    m.add_class::<#handle_classes>()?;
                )*
                Ok(())
            }
        }
    };

    vec![GeneratedFile {
        name: "pyo3_bridge.rs".to_string(),
        content: pretty_print(tokens),
    }]
}
