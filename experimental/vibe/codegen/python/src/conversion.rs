// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use convert_case::Case;
use proc_macro2::TokenStream;
use quent_ref_target::RefTarget;
use quent_schema::{DataType, Schema};
use quote::{format_ident, quote};

use crate::GenerateError;
use crate::common::{path_pascal, py_safe, raw_ident, rust_path, to_case};

pub(crate) fn convert(
    schema: &Schema,
    ty: &DataType,
    expression: TokenStream,
    instrumentation: &syn::Path,
    runtime: &syn::Path,
) -> Result<TokenStream, GenerateError> {
    Ok(match ty {
        DataType::Bool => quote! { (#expression).extract::<bool>()? },
        DataType::String => quote! { (#expression).extract::<String>()? },
        DataType::U8 => quote! { (#expression).extract::<u8>()? },
        DataType::U16 => quote! { (#expression).extract::<u16>()? },
        DataType::U32 => quote! { (#expression).extract::<u32>()? },
        DataType::U64 => quote! { (#expression).extract::<u64>()? },
        DataType::I8 => quote! { (#expression).extract::<i8>()? },
        DataType::I16 => quote! { (#expression).extract::<i16>()? },
        DataType::I32 => quote! { (#expression).extract::<i32>()? },
        DataType::I64 => quote! { (#expression).extract::<i64>()? },
        DataType::F32 => quote! { (#expression).extract::<f32>()? },
        DataType::F64 => quote! { (#expression).extract::<f64>()? },
        DataType::Uuid => quote! { __extract_uuid(#expression)? },
        DataType::DynamicRecord => quote! { __extract_dynamic_attributes(#expression)? },
        DataType::Option(inner) => {
            let value = convert(schema, inner, expression.clone(), instrumentation, runtime)?;
            quote! { if (#expression).is_none() { None } else { Some(#value) } }
        }
        DataType::List(inner) => {
            let value = convert(schema, inner, quote! { &item }, instrumentation, runtime)?;
            quote! {{
                (#expression).try_iter()?
                    .map(|item| {
                        let item = item?;
                        let value = #value;
                        Ok(value)
                    })
                    .collect::<PyResult<Vec<_>>>()?
            }}
        }
        DataType::Record(path) => {
            let target = rust_path(instrumentation, path, "");
            let record = schema.record(path).expect("validated record reference");
            let fields = record
                .fields()
                .map(|field| {
                    let rust_name = raw_ident(to_case(field.name(), Case::Snake));
                    let key = py_safe(&to_case(field.name(), Case::Snake));
                    let value = convert(
                        schema,
                        field.ty(),
                        quote! { &field_value },
                        instrumentation,
                        runtime,
                    )?;
                    Ok::<_, GenerateError>(quote! {
                        #rust_name: {
                            let field_value = values.get_item(#key)?;
                            #value
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if fields.is_empty() {
                quote! {{ let _ = #expression; #target }}
            } else {
                quote! {{
                    let values = (#expression).cast::<PyMapping>().map_err(|_| {
                        pyo3::exceptions::PyTypeError::new_err("expected mapping for record")
                    })?;
                    #target { #(#fields),* }
                }}
            }
        }
        DataType::EntityRef { data, annotations } => {
            let target = RefTarget::from_annotations(annotations);
            let marker = target
                .as_ref()
                .map(|target| rust_path(instrumentation, target.as_ref(), ""))
                .unwrap_or_else(|| quote! { #instrumentation::AnyEntity });
            let extract_id = if let Some(target) = &target {
                let handle = format_ident!("Py{}Handle", path_pascal(target.as_ref()));
                quote! {{
                    if let Ok(value) = target_value.extract::<PyRef<'_, #handle>>() {
                        value.inner.uuid()
                    } else {
                        __extract_uuid(&target_value)?
                    }
                }}
            } else {
                quote! { __extract_uuid(&target_value)? }
            };
            match data {
                None => quote! {{
                    let target_value = (#expression).clone();
                    #runtime::EntityRef::<#marker>::new(#extract_id, ())
                }},
                Some(data) => {
                    let converted = convert(
                        schema,
                        data,
                        quote! { &data_value },
                        instrumentation,
                        runtime,
                    )?;
                    quote! {{
                        let values = (#expression).cast::<PyMapping>().map_err(|_| {
                            pyo3::exceptions::PyTypeError::new_err(
                                "expected mapping with `target` and `data` for entity reference",
                            )
                        })?;
                        let target_value = values.get_item("target")?;
                        let data_value = values.get_item("data")?;
                        #runtime::EntityRef::<#marker, _>::new(#extract_id, #converted)
                    }}
                }
            }
        }
    })
}
