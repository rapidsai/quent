// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_ref_target::RefTarget;
use quent_schema::{Cardinality, DataType, Entity, Path, Schema};
use quote::{format_ident, quote};

use crate::common::{
    cxx_namespace, cxx_safe, path_pascal, path_snake, pretty, raw_ident, rust_path, to_case,
};
use crate::{GenerateError, GeneratedFile, Options};

pub(crate) fn entity_file(
    schema: &Schema,
    entity: &Entity,
    options: &Options,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
) -> Result<GeneratedFile, GenerateError> {
    let mut registry = TypeRegistry::new(schema, instrumentation, runtime);
    let mut payloads = Vec::new();
    for event in entity.events() {
        if event.fields().next().is_none() {
            continue;
        }
        let name = to_case(event.name(), Case::Pascal);
        let mut fields = Vec::new();
        for field in event.fields() {
            let ty =
                registry.cxx_type(field.ty(), &format!("{}.{}", entity.path(), field.name()))?;
            fields.push((cxx_safe(&to_case(field.name(), Case::Snake)), ty));
        }
        registry.reserve(&name, &format!("event:{}.{}", entity.path(), event.name()))?;
        payloads.push(render_struct(&name, &fields));
    }

    let entity_name = path_pascal(entity.path());
    let observer_name = format!("{entity_name}Observer");
    let handle_name = format!("{entity_name}Handle");
    let detail_namespace = format!("{}::detail", options.namespace);
    let namespace = cxx_namespace(&detail_namespace, entity.path());
    let uuid_include = format!("{}/{}/uuid.rs.h", options.crate_name, options.bridge_path);
    let context_include = format!(
        "{}/{}/context.rs.h",
        options.crate_name, options.bridge_path
    );
    let dynamic_include = format!(
        "{}/{}/dynamic_attributes.rs.h",
        options.crate_name, options.bridge_path
    );

    let mut extern_body = format!(
        "        type {observer_name};\n        type {handle_name};\n\n\
         fn create_observer(ctx: &Context) -> Box<{observer_name}>;\n\
         fn create(self: &{observer_name}) -> Box<{handle_name}>;\n\
         fn create_with_id(self: &{observer_name}, id: UUID) -> Box<{handle_name}>;\n\
         fn uuid(self: &{handle_name}) -> UUID;\n"
    );

    let entity_ty = rust_path(instrumentation, entity.path(), "");
    let observer_ident = format_ident!("{observer_name}");
    let handle_ident = format_ident!("{handle_name}");
    let mut methods = Vec::new();
    for event in entity.events() {
        let rust_method = raw_ident(to_case(event.name(), Case::Snake));
        let payload_name = format_ident!("{}", to_case(event.name(), Case::Pascal));
        let exported = cxx_safe(&to_case(event.name(), Case::Snake));
        let cxx_name = if rust_method != exported {
            format!("        #[cxx_name = \"{exported}\"]\n")
        } else {
            String::new()
        };
        let receiver = match event.cardinality() {
            Cardinality::Once => format!("self: &mut {handle_name}"),
            Cardinality::Multi => format!("self: &{handle_name}"),
        };
        let call_args = event
            .fields()
            .map(|field| {
                let ffi_name = raw_ident(cxx_safe(&to_case(field.name(), Case::Snake)));
                registry.convert(field.ty(), quote! { data.#ffi_name })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let implementation = if event.cardinality() == Cardinality::Once {
            quote! {
                pub fn #rust_method(&mut self) -> Result<(), String> {
                    self.inner.#rust_method().map_err(|error| error.to_string())
                }
            }
        } else {
            quote! {
                pub fn #rust_method(&self) -> Result<(), String> {
                    self.inner.#rust_method().map_err(|error| error.to_string())
                }
            }
        };
        let implementation_with_data = if event.cardinality() == Cardinality::Once {
            quote! {
                pub fn #rust_method(&mut self, data: ffi::#payload_name) -> Result<(), String> {
                    self.inner.#rust_method(#(#call_args),*)
                        .map_err(|error| error.to_string())
                }
            }
        } else {
            quote! {
                pub fn #rust_method(&self, data: ffi::#payload_name) -> Result<(), String> {
                    self.inner.#rust_method(#(#call_args),*)
                        .map_err(|error| error.to_string())
                }
            }
        };
        if event.fields().next().is_none() {
            extern_body.push_str(&format!(
                "{cxx_name}        fn {}({receiver}) -> Result<()>;\n",
                rust_method
            ));
            methods.push(implementation);
        } else {
            extern_body.push_str(&format!(
                "{cxx_name}        fn {}({receiver}, data: {}) -> Result<()>;\n",
                rust_method, payload_name
            ));
            methods.push(implementation_with_data);
        }
        if event.cardinality() == Cardinality::Once {
            let emitted_name = format!("{}_emitted", to_case(event.name(), Case::Snake));
            let emitted = raw_ident(&emitted_name);
            let exported_emitted = cxx_safe(&emitted_name);
            let emitted_cxx_name = if emitted != exported_emitted {
                format!("        #[cxx_name = \"{exported_emitted}\"]\n")
            } else {
                String::new()
            };
            extern_body.push_str(&format!(
                "{emitted_cxx_name}        fn {emitted}(self: &{handle_name}) -> bool;\n"
            ));
            methods.push(quote! {
                pub fn #emitted(&self) -> bool { self.inner.#emitted() }
            });
        }
    }

    let aliases = dynamic_aliases(&dynamic_include, &detail_namespace);
    let definitions = format!("{}{}", registry.definitions.join(""), payloads.join(""));
    let base_namespace = &detail_namespace;
    let uuid_namespace = format!("{detail_namespace}::uuid");
    let ffi = format!(
        r#"#[cxx::bridge(namespace = "{namespace}")]
pub mod ffi {{
    unsafe extern "C++" {{ include!("rust/cxx.h"); }}
    #[namespace = "{uuid_namespace}"]
    unsafe extern "C++" {{
        include!("{uuid_include}");
        type UUID = crate::bridge::uuid::ffi::UUID;
    }}
    #[namespace = "{base_namespace}"]
    unsafe extern "C++" {{
        include!("{context_include}");
        type Context = crate::bridge::context::Context;
    }}
{aliases}
{definitions}    extern "Rust" {{
{extern_body}    }}
}}
"#
    );

    let tokens = quote! {
        pub struct #observer_ident { inner: #instrumentation::Observer<#entity_ty> }
        pub struct #handle_ident { inner: #instrumentation::Handle<#entity_ty> }

        pub fn create_observer(ctx: &super::context::Context) -> Box<#observer_ident> {
            Box::new(#observer_ident { inner: ctx.inner.observer::<#entity_ty>() })
        }

        impl #observer_ident {
            pub fn create(&self) -> Box<#handle_ident> {
                Box::new(#handle_ident { inner: self.inner.handle() })
            }
            pub fn create_with_id(&self, id: ffi::UUID) -> Box<#handle_ident> {
                Box::new(#handle_ident { inner: self.inner.handle_with_id(#runtime::Uuid::from(id)) })
            }
        }

        impl #handle_ident {
            pub fn uuid(&self) -> ffi::UUID { self.inner.uuid().into() }
            #(#methods)*
        }
    };

    Ok(GeneratedFile {
        name: format!("{}.rs", path_snake(entity.path())),
        content: format!("{ffi}\n{}", pretty(tokens)?),
    })
}

fn dynamic_aliases(include: &str, namespace: &str) -> String {
    format!(
        r#"    #[namespace = "{namespace}"]
    unsafe extern "C++" {{
        include!("{include}");
        type DynamicAttributeKind = crate::bridge::dynamic_attributes::ffi::DynamicAttributeKind;
        type DynamicAttribute = crate::bridge::dynamic_attributes::ffi::DynamicAttribute;
        type DynamicAttributes = crate::bridge::dynamic_attributes::ffi::DynamicAttributes;
    }}
"#
    )
}

fn render_struct(name: &str, fields: &[(String, String)]) -> String {
    let fields = fields
        .iter()
        .map(|(name, ty)| format!("        pub {name}: {ty},\n"))
        .collect::<String>();
    format!("    #[derive(Debug, Default)]\n    pub struct {name} {{\n{fields}    }}\n\n")
}

struct TypeRegistry<'a> {
    schema: &'a Schema,
    instrumentation: &'a syn::Path,
    runtime: &'a syn::Path,
    definitions: Vec<String>,
    names: BTreeMap<String, String>,
}

impl<'a> TypeRegistry<'a> {
    fn new(schema: &'a Schema, instrumentation: &'a syn::Path, runtime: &'a syn::Path) -> Self {
        Self {
            schema,
            instrumentation,
            runtime,
            definitions: Vec::new(),
            names: BTreeMap::new(),
        }
    }

    fn reserve(&mut self, name: &str, identity: &str) -> Result<bool, GenerateError> {
        match self.names.get(name) {
            Some(existing) if existing == identity => Ok(false),
            Some(_) => Err(GenerateError::NameCollision {
                name: name.to_owned(),
            }),
            None => {
                self.names.insert(name.to_owned(), identity.to_owned());
                Ok(true)
            }
        }
    }

    fn cxx_type(&mut self, ty: &DataType, location: &str) -> Result<String, GenerateError> {
        Ok(match ty {
            DataType::Bool => "bool".to_owned(),
            DataType::Uuid => "UUID".to_owned(),
            DataType::String => "String".to_owned(),
            DataType::U8 => "u8".to_owned(),
            DataType::U16 => "u16".to_owned(),
            DataType::U32 => "u32".to_owned(),
            DataType::U64 => "u64".to_owned(),
            DataType::I8 => "i8".to_owned(),
            DataType::I16 => "i16".to_owned(),
            DataType::I32 => "i32".to_owned(),
            DataType::I64 => "i64".to_owned(),
            DataType::F32 => "f32".to_owned(),
            DataType::F64 => "f64".to_owned(),
            DataType::DynamicRecord => "DynamicAttributes".to_owned(),
            DataType::Record(path) => self.record(path, location)?,
            DataType::Option(inner) => self.optional(inner, location)?,
            DataType::List(inner) => {
                if matches!(inner.as_ref(), DataType::List(_)) {
                    let wrapper = format!("BridgeList{}", self.type_key(inner));
                    let identity = format!("list:{inner:?}");
                    if self.reserve(&wrapper, &identity)? {
                        let values = self.cxx_type(inner, location)?;
                        self.definitions
                            .push(render_struct(&wrapper, &[("values".to_owned(), values)]));
                    }
                    format!("Vec<{wrapper}>")
                } else {
                    format!("Vec<{}>", self.cxx_type(inner, location)?)
                }
            }
            DataType::EntityRef { data: None, .. } => "UUID".to_owned(),
            DataType::EntityRef {
                data: Some(data),
                annotations,
            } => {
                let target = RefTarget::from_annotations(annotations)
                    .map(|target| path_pascal(target.as_ref()))
                    .unwrap_or_else(|| "AnyEntity".to_owned());
                let name = format!("BridgeReference{target}{}", self.type_key(data));
                let identity = format!("reference:{ty:?}");
                if self.reserve(&name, &identity)? {
                    let data_ty = self.cxx_type(data, location)?;
                    self.definitions.push(render_struct(
                        &name,
                        &[
                            ("target".to_owned(), "UUID".to_owned()),
                            ("data".to_owned(), data_ty),
                        ],
                    ));
                }
                name
            }
        })
    }

    fn record(&mut self, path: &Path, location: &str) -> Result<String, GenerateError> {
        let name = format!("BridgeRecord{}", path_pascal(path));
        let identity = format!("record:{path}");
        if !self.reserve(&name, &identity)? {
            return Ok(name);
        }
        let record = self
            .schema
            .record(path)
            .ok_or_else(|| GenerateError::UnsupportedType {
                location: location.to_owned(),
                reason: format!("record `{path}` does not exist"),
            })?;
        let fields = record
            .fields()
            .map(|field| (field.name().to_string(), field.ty().clone()))
            .collect::<Vec<_>>();
        let mut rendered = Vec::new();
        for (field_name, ty) in fields {
            let field_location = format!("{path}.{field_name}");
            rendered.push((
                cxx_safe(&to_case(&field_name, Case::Snake)),
                self.cxx_type(&ty, &field_location)?,
            ));
        }
        if rendered.is_empty() {
            rendered.push(("unit".to_owned(), "bool".to_owned()));
        }
        self.definitions.push(render_struct(&name, &rendered));
        Ok(name)
    }

    fn optional(&mut self, inner: &DataType, location: &str) -> Result<String, GenerateError> {
        let name = format!("BridgeOptional{}", self.type_key(inner));
        let identity = format!("optional:{inner:?}");
        if self.reserve(&name, &identity)? {
            let value = self.cxx_type(inner, location)?;
            self.definitions.push(render_struct(
                &name,
                &[
                    ("has_value".to_owned(), "bool".to_owned()),
                    ("value".to_owned(), value),
                ],
            ));
        }
        Ok(name)
    }

    fn type_key(&self, ty: &DataType) -> String {
        match ty {
            DataType::Bool => "Bool".to_owned(),
            DataType::Uuid => "Uuid".to_owned(),
            DataType::String => "String".to_owned(),
            DataType::U8 => "U8".to_owned(),
            DataType::U16 => "U16".to_owned(),
            DataType::U32 => "U32".to_owned(),
            DataType::U64 => "U64".to_owned(),
            DataType::I8 => "I8".to_owned(),
            DataType::I16 => "I16".to_owned(),
            DataType::I32 => "I32".to_owned(),
            DataType::I64 => "I64".to_owned(),
            DataType::F32 => "F32".to_owned(),
            DataType::F64 => "F64".to_owned(),
            DataType::Option(inner) => format!("Optional{}", self.type_key(inner)),
            DataType::List(inner) => format!("{}List", self.type_key(inner)),
            DataType::Record(path) => path_pascal(path),
            DataType::DynamicRecord => "DynamicAttributes".to_owned(),
            DataType::EntityRef { data, annotations } => {
                let target = RefTarget::from_annotations(annotations)
                    .map(|target| path_pascal(target.as_ref()))
                    .unwrap_or_else(|| "AnyEntity".to_owned());
                match data {
                    Some(data) => {
                        let data = self.type_key(data);
                        if data
                            .strip_prefix(&target)
                            .is_some_and(|suffix| !suffix.is_empty())
                        {
                            format!("{data}Ref")
                        } else {
                            format!("{target}{data}Ref")
                        }
                    }
                    None => format!("{target}Ref"),
                }
            }
        }
    }

    fn convert(
        &self,
        ty: &DataType,
        expression: TokenStream,
    ) -> Result<TokenStream, GenerateError> {
        Ok(match ty {
            DataType::Bool
            | DataType::String
            | DataType::U8
            | DataType::U16
            | DataType::U32
            | DataType::U64
            | DataType::I8
            | DataType::I16
            | DataType::I32
            | DataType::I64
            | DataType::F32
            | DataType::F64 => expression,
            DataType::Uuid => {
                let runtime = self.runtime;
                quote! { #runtime::Uuid::from(#expression) }
            }
            DataType::DynamicRecord => quote! { (#expression).into_model() },
            DataType::Option(inner) => {
                let converted = self.convert(inner, quote! { value.value })?;
                quote! {{
                    let value = #expression;
                    if value.has_value { Some(#converted) } else { None }
                }}
            }
            DataType::List(inner) => {
                let item = if matches!(inner.as_ref(), DataType::List(_)) {
                    quote! { value.values }
                } else {
                    quote! { value }
                };
                let converted = self.convert(inner, item)?;
                quote! { (#expression).into_iter().map(|value| #converted).collect() }
            }
            DataType::Record(path) => {
                let target = rust_path(self.instrumentation, path, "");
                let record = self
                    .schema
                    .record(path)
                    .expect("validated record reference");
                let fields = record
                    .fields()
                    .map(|field| {
                        let model_name = raw_ident(to_case(field.name(), Case::Snake));
                        let ffi_name = raw_ident(cxx_safe(&to_case(field.name(), Case::Snake)));
                        let value = self.convert(field.ty(), quote! { value.#ffi_name })?;
                        Ok::<_, GenerateError>(quote! { #model_name: #value })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if fields.is_empty() {
                    quote! { #target }
                } else {
                    quote! {{ let value = #expression; #target { #(#fields),* } }}
                }
            }
            DataType::EntityRef { data, annotations } => {
                let target = RefTarget::from_annotations(annotations)
                    .map(|target| rust_path(self.instrumentation, target.as_ref(), ""))
                    .unwrap_or_else(|| {
                        let instrumentation = self.instrumentation;
                        quote! { #instrumentation::AnyEntity }
                    });
                let runtime = self.runtime;
                match data {
                    None => quote! {
                        #runtime::EntityRef::<#target>::new(#runtime::Uuid::from(#expression), ())
                    },
                    Some(data) => {
                        let converted = self.convert(data, quote! { value.data })?;
                        quote! {{
                            let value = #expression;
                            #runtime::EntityRef::<#target, _>::new(
                                #runtime::Uuid::from(value.target),
                                #converted,
                            )
                        }}
                    }
                }
            }
        })
    }
}
