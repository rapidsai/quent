// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! CXX bridge code generator.
//!
//! Generates Rust `#[cxx::bridge]` modules from model definitions. The output
//! is Rust source code that CXX compiles into C++ headers.
//!
//! Uses `quote!` to build token streams and `prettyplease` for formatting.
//! The `#[cxx::bridge]` `ffi` module itself is built as a formatted string
//! because CXX bridge syntax (e.g. `type Alias = path;` in extern blocks)
//! is not representable in standard Rust AST and cannot be formatted by
//! `prettyplease`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use quent_model::{AttributeDef, FsmDef, ModelBuilder, StateDef, ValueType};

use crate::{CxxOptions, GeneratedFile};

/// Recursively check whether any attribute in the list (or nested structs) uses `CustomAttributes`.
fn attrs_use_custom_attributes(attrs: &[AttributeDef]) -> bool {
    attrs.iter().any(|a| match &a.value_type {
        ValueType::CustomAttributes => true,
        ValueType::Struct(_, inner) => attrs_use_custom_attributes(inner),
        ValueType::List(inner) => match inner.as_ref() {
            ValueType::Struct(_, inner_attrs) => attrs_use_custom_attributes(inner_attrs),
            _ => false,
        },
        _ => false,
    })
}

/// C++ reserved keywords that cannot be used as namespace names.
const CXX_RESERVED_KEYWORDS: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
    "auto",
    "bitand",
    "bitor",
    "bool",
    "break",
    "case",
    "catch",
    "char",
    "char8_t",
    "char16_t",
    "char32_t",
    "class",
    "compl",
    "concept",
    "const",
    "consteval",
    "constexpr",
    "constinit",
    "const_cast",
    "continue",
    "co_await",
    "co_return",
    "co_yield",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "dynamic_cast",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "float",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "not",
    "not_eq",
    "nullptr",
    "operator",
    "or",
    "or_eq",
    "private",
    "protected",
    "public",
    "register",
    "reinterpret_cast",
    "requires",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "static_cast",
    "struct",
    "switch",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "try",
    "typedef",
    "typeid",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
    "wchar_t",
    "while",
    "xor",
    "xor_eq",
];

/// Sanitize a name to avoid C++ reserved keywords by appending an underscore.
fn cxx_safe_name(name: &str) -> String {
    if CXX_RESERVED_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// Parse the `__quent` re-export path from options: `{instrumentation_crate}::__quent`.
fn quent_path(options: &CxxOptions) -> syn::Path {
    syn::parse_str(&format!("{}::__quent", options.instrumentation_crate)).unwrap()
}

/// Map a Quent `ValueType` to a CXX-compatible Rust type string.
/// Map a Quent `ValueType` to a CXX-compatible Rust type string.
/// Returns None if the type is not representable in CXX.
fn value_type_to_cxx(ty: &ValueType, optional: bool) -> Option<String> {
    let base = match ty {
        ValueType::Bool => "bool".to_string(),
        ValueType::Uuid => "UUID".to_string(),
        ValueType::String => "String".to_string(),
        ValueType::U8 => "u8".to_string(),
        ValueType::U16 => "u16".to_string(),
        ValueType::U32 => "u32".to_string(),
        ValueType::U64 => "u64".to_string(),
        ValueType::I8 => "i8".to_string(),
        ValueType::I16 => "i16".to_string(),
        ValueType::I32 => "i32".to_string(),
        ValueType::I64 => "i64".to_string(),
        ValueType::F32 => "f32".to_string(),
        ValueType::F64 => "f64".to_string(),
        ValueType::Ref(_) => "UUID".to_string(),
        ValueType::CustomAttributes => "CustomAttributes".to_string(),
        ValueType::List(inner) => {
            let inner_cxx = value_type_to_cxx(inner, false)?;
            format!("Vec<{inner_cxx}>")
        }
        // Nested structs are handled by generating separate shared structs.
        ValueType::Struct(_, _) => return None,
    };
    // For optional types, CXX uses the base type with sentinels:
    // Option<Ref<T>> → UUID (nil = None)
    // Option<String> → String (empty = None)
    // Other optional types are not supported
    if optional {
        match ty {
            ValueType::Ref(_) | ValueType::Uuid | ValueType::String => Some(base),
            _ => None,
        }
    } else {
        Some(base)
    }
}

/// Convert snake_case to PascalCase.
fn to_pascal_case(s: &str) -> String {
    use convert_case::{Case, Casing};
    s.to_case(Case::Pascal)
}

/// Format a `TokenStream` into a pretty-printed Rust source string via `prettyplease`.
fn pretty_print(tokens: TokenStream) -> String {
    let file = syn::parse2::<syn::File>(tokens).expect("generated tokens must be valid syntax");
    prettyplease::unparse(&file)
}

/// Generate CXX bridge files for all model components.
pub fn emit(model: &ModelBuilder, options: &CxxOptions) -> Vec<GeneratedFile> {
    let mut files = Vec::new();

    // Generate UUID bridge (shared type used by all bridges)
    files.push(emit_uuid_bridge(model, options));

    // Generate custom attributes bridge if any component uses CustomAttributes
    let uses_custom_attrs = model.entities.iter().any(|e| {
        e.events
            .iter()
            .any(|ev| attrs_use_custom_attributes(&ev.attributes))
    }) || model.fsms.iter().any(|f| {
        f.states
            .iter()
            .any(|s| attrs_use_custom_attributes(&s.attributes))
    });
    if uses_custom_attrs {
        files.push(emit_custom_attributes_bridge(options));
    }

    // Generate context bridge
    files.push(emit_context_bridge(options));

    // Generate entity bridges
    for entity in &model.entities {
        files.push(emit_entity_bridge(entity, options));
    }

    // Generate FSM bridges
    for fsm in &model.fsms {
        files.push(emit_fsm_bridge(fsm, options));
    }

    // Generate lib.rs that includes all modules
    files.push(emit_lib_rs(model, options));

    files
}

/// Generate the CustomAttributes CXX shared type bridge.
///
/// Uses CXX shared structs (one per value type) that C++ can construct
/// natively. A Rust conversion function assembles them into
/// `quent_attributes::CustomAttributes`.
fn emit_custom_attributes_bridge(options: &CxxOptions) -> GeneratedFile {
    let q = quent_path(options);

    let tokens = quote! {
        #[cxx::bridge(namespace = "quent")]
        pub mod ffi {
            unsafe extern "C++" {
                include!("rust/cxx.h");
            }

            #[derive(Debug, Default)]
            pub struct StringAttr {
                pub key: String,
                pub value: String,
            }

            #[derive(Debug, Default)]
            pub struct I64Attr {
                pub key: String,
                pub value: i64,
            }

            #[derive(Debug, Default)]
            pub struct F64Attr {
                pub key: String,
                pub value: f64,
            }

            #[derive(Debug, Default)]
            pub struct CustomAttributes {
                pub string_attrs: Vec<StringAttr>,
                pub i64_attrs: Vec<I64Attr>,
                pub f64_attrs: Vec<F64Attr>,
            }
        }

        impl ffi::CustomAttributes {
            pub fn into_model(self) -> #q::attributes::CustomAttributes {
                let mut attrs = #q::attributes::CustomAttributes::new();
                for a in self.string_attrs {
                    attrs.add_string(a.key, a.value);
                }
                for a in self.i64_attrs {
                    attrs.add_i64(a.key, a.value);
                }
                for a in self.f64_attrs {
                    attrs.add_f64(a.key, a.value);
                }
                attrs
            }
        }
    };

    GeneratedFile {
        name: "custom_attributes.rs".to_string(),
        content: pretty_print(tokens),
    }
}

/// Generate the UUID shared type bridge.
fn emit_uuid_bridge(model: &ModelBuilder, options: &CxxOptions) -> GeneratedFile {
    let q = quent_path(options);
    // Check if any model component uses Vec<UUID> (Vec<Ref<_>> or Vec<Uuid>)
    let needs_vec_uuid = model.entities.iter().any(|e| {
        e.events.iter().any(|ev| {
            ev.attributes.iter().any(|a| {
                matches!(
                    &a.value_type,
                    ValueType::List(inner) if matches!(inner.as_ref(), ValueType::Ref(_) | ValueType::Uuid)
                )
            })
        })
    }) || model.fsms.iter().any(|f| {
        f.states.iter().any(|s| {
            s.attributes.iter().any(|a| {
                matches!(
                    &a.value_type,
                    ValueType::List(inner) if matches!(inner.as_ref(), ValueType::Ref(_) | ValueType::Uuid)
                )
            })
        })
    });

    // If Vec<UUID> is needed in other bridge modules, expose a dummy function
    // in the uuid bridge so CXX generates the ImplVec trait for UUID.
    let (vec_uuid_ffi, vec_uuid_impl) = if needs_vec_uuid {
        (
            quote! {
                fn uuid_vec_noop(_v: &Vec<UUID>);
            },
            quote! {
                #[allow(unused)]
                fn uuid_vec_noop(_v: &Vec<ffi::UUID>) {}
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    let tokens = quote! {
        #[cxx::bridge(namespace = "uuid")]
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

                #vec_uuid_ffi
            }
        }

        #vec_uuid_impl

        fn uuid_now_v7() -> ffi::UUID {
            let id = #q::uuid::Uuid::now_v7();
            let (high, low) = id.as_u64_pair();
            ffi::UUID {
                high_bits: high,
                low_bits: low,
            }
        }

        fn uuid_new_nil() -> ffi::UUID {
            ffi::UUID {
                high_bits: 0,
                low_bits: 0,
            }
        }

        impl From<ffi::UUID> for #q::uuid::Uuid {
            fn from(u: ffi::UUID) -> Self {
                #q::uuid::Uuid::from_u64_pair(u.high_bits, u.low_bits)
            }
        }

        impl From<#q::uuid::Uuid> for ffi::UUID {
            fn from(u: #q::uuid::Uuid) -> Self {
                let (high, low) = u.as_u64_pair();
                ffi::UUID {
                    high_bits: high,
                    low_bits: low,
                }
            }
        }
    };

    GeneratedFile {
        name: "uuid.rs".to_string(),
        content: pretty_print(tokens),
    }
}

/// Generate the context bridge module.
///
/// The context is created once and stores the event sender in a global static.
/// This avoids the need to share opaque Rust types across CXX bridge modules.
fn emit_context_bridge(options: &CxxOptions) -> GeneratedFile {
    let ns = &options.namespace;
    let q = quent_path(options);
    let event_type: syn::Type = syn::parse_str(&options.event_type()).unwrap();

    let tokens = quote! {
        use std::sync::OnceLock;

        #[cxx::bridge(namespace = #ns)]
        pub mod ffi {
            unsafe extern "C++" {
                include!("rust/cxx.h");
            }

            extern "Rust" {
                type Context;
                fn create_context(exporter: String, output_dir: String) -> Result<Box<Context>>;
            }
        }

        /// Global event sender, initialized by `create_context`.
        /// Returns a noop sender if context has not been created yet.
        static SENDER: OnceLock<#q::EventSender<#event_type>> = OnceLock::new();

        pub struct Context {
            _inner: #q::Context<#event_type>,
        }

        pub fn global_sender() -> #q::EventSender<#event_type> {
            SENDER
                .get()
                .cloned()
                .unwrap_or_default()
        }

        pub fn create_context(exporter: String, output_dir: String) -> Result<Box<Context>, String> {
            let opts = match exporter.as_str() {
                "ndjson" => Some(#q::exporter::ExporterOptions::Ndjson(
                    #q::exporter::NdjsonExporterOptions {
                        output_dir: output_dir.into(),
                    },
                )),
                _ => None,
            };
            let inner = #q::Context::try_new(opts, #q::uuid::Uuid::now_v7())
                .map_err(|e| e.to_string())?;
            let _ = SENDER.set(inner.events_sender());
            Ok(Box::new(Context { _inner: inner }))
        }
    };

    GeneratedFile {
        name: "context.rs".to_string(),
        content: pretty_print(tokens),
    }
}

/// Build the `#[cxx::bridge] pub mod ffi { ... }` block as a formatted string.
///
/// CXX bridge syntax contains constructs like `type UUID = crate::path;` inside
/// `unsafe extern "C++"` blocks that are not standard Rust. `prettyplease` cannot
/// format these, so the ffi module is built as a string.
fn build_ffi_module_string(
    ns: &str,
    include_path: &str,
    shared_structs: &str,
    extern_rust_body: &str,
    uses_custom_attrs: bool,
) -> String {
    let ca_include = include_path.replace("uuid.rs.h", "custom_attributes.rs.h");
    let custom_attrs_types = if uses_custom_attrs {
        format!(
            r#"
    #[namespace = "quent"]
    unsafe extern "C++" {{
        include!("{ca_include}");
        type StringAttr = crate::bridge::custom_attributes::ffi::StringAttr;
        type I64Attr = crate::bridge::custom_attributes::ffi::I64Attr;
        type F64Attr = crate::bridge::custom_attributes::ffi::F64Attr;
        type CustomAttributes = crate::bridge::custom_attributes::ffi::CustomAttributes;
    }}
"#
        )
    } else {
        String::new()
    };

    format!(
        r#"#[cxx::bridge(namespace = "{ns}")]
pub mod ffi {{
    unsafe extern "C++" {{
        include!("rust/cxx.h");
    }}

    #[namespace = "uuid"]
    unsafe extern "C++" {{
        include!("{include_path}");
        type UUID = crate::bridge::uuid::ffi::UUID;
    }}
{custom_attrs_types}
{shared_structs}    extern "Rust" {{
{extern_rust_body}    }}
}}
"#
    )
}

/// Generate a CXX shared struct definition string for a set of attributes.
/// Recursively generates nested struct definitions for `ValueType::Struct` fields.
/// Returns (field definitions string, additional struct definitions string).
fn generate_cxx_struct_fields(attrs: &[AttributeDef], parent_name: &str) -> (String, String) {
    let mut fields_str = String::new();
    let mut nested_structs = String::new();

    for attr in attrs {
        if let ValueType::Struct(_, inner_attrs) = &attr.value_type {
            // Generate a nested struct with PascalCase name from the field name
            let nested_name = to_pascal_case(&attr.name);
            let (inner_fields, more_nested) = generate_cxx_struct_fields(inner_attrs, &nested_name);
            nested_structs.push_str(&more_nested);
            nested_structs.push_str(&format!(
                "    #[derive(Debug, Default)]\n    pub struct {nested_name} {{\n{inner_fields}    }}\n\n"
            ));

            if attr.optional {
                // Optional nested struct: include a has_ flag
                fields_str.push_str(&format!("        pub has_{}: bool,\n", attr.name));
            }

            // Vec<Struct> or plain struct
            if let ValueType::List(_) = &attr.value_type {
                fields_str.push_str(&format!(
                    "        pub {}: Vec<{}>,\n",
                    attr.name, nested_name
                ));
            } else {
                fields_str.push_str(&format!("        pub {}: {},\n", attr.name, nested_name));
            }
        } else if let ValueType::List(inner) = &attr.value_type {
            if let ValueType::Struct(_, inner_attrs) = inner.as_ref() {
                let nested_name = to_pascal_case(&attr.name);
                let (inner_fields, more_nested) =
                    generate_cxx_struct_fields(inner_attrs, &nested_name);
                nested_structs.push_str(&more_nested);
                nested_structs.push_str(&format!(
                    "    #[derive(Debug, Default)]\n    pub struct {nested_name} {{\n{inner_fields}    }}\n\n"
                ));
                fields_str.push_str(&format!(
                    "        pub {}: Vec<{}>,\n",
                    attr.name, nested_name
                ));
            } else {
                let cxx_type =
                    value_type_to_cxx(&attr.value_type, attr.optional).unwrap_or_else(|| {
                        panic!(
                            "field `{}` on `{}` has type not representable in CXX",
                            attr.name, parent_name,
                        )
                    });
                fields_str.push_str(&format!("        pub {}: {},\n", attr.name, cxx_type));
            }
        } else {
            let cxx_type =
                value_type_to_cxx(&attr.value_type, attr.optional).unwrap_or_else(|| {
                    panic!(
                        "field `{}` on `{}` has type not representable in CXX",
                        attr.name, parent_name,
                    )
                });
            fields_str.push_str(&format!("        pub {}: {},\n", attr.name, cxx_type));
        }
    }

    (fields_str, nested_structs)
}

/// Generate a field conversion expression for an attribute: `name: <conversion>(data.name)`.
/// `component_mod` is the path prefix for model types (e.g., `icrate::engine`).
fn emit_field_conversion_tokens(
    attr: &AttributeDef,
    q: &syn::Path,
    component_mod: &syn::Path,
) -> TokenStream {
    let name = format_ident!("{}", attr.name);

    if attr.optional {
        // Optional field: CXX uses sentinels
        return match &attr.value_type {
            ValueType::Ref(_) | ValueType::Uuid => quote! {
                #name: {
                    let uuid = #q::uuid::Uuid::from(data.#name);
                    if uuid.is_nil() { None } else { Some(#q::Ref::new(uuid)) }
                },
            },
            ValueType::String => quote! {
                #name: if data.#name.is_empty() { None } else { Some(data.#name) },
            },
            _ => quote! {
                #name: data.#name,
            },
        };
    }

    match &attr.value_type {
        ValueType::Uuid => quote! {
            #name: #q::uuid::Uuid::from(data.#name),
        },
        ValueType::Ref(_) => quote! {
            #name: #q::Ref::new(#q::uuid::Uuid::from(data.#name)),
        },
        ValueType::CustomAttributes => quote! {
            #name: data.#name.into_model(),
        },
        ValueType::Struct(type_path, inner_attrs) => {
            let conversion = emit_struct_conversion(type_path, inner_attrs, q, component_mod);
            quote! {
                #name: {
                    let data = data.#name;
                    #conversion
                },
            }
        }
        ValueType::List(inner) => match inner.as_ref() {
            ValueType::Ref(_) => quote! {
                #name: data.#name.into_iter().map(|u| #q::Ref::new(#q::uuid::Uuid::from(u))).collect(),
            },
            ValueType::Uuid => quote! {
                #name: data.#name.into_iter().map(|u| #q::uuid::Uuid::from(u)).collect(),
            },
            ValueType::Struct(type_path, inner_attrs) => {
                let conversion = emit_struct_conversion(type_path, inner_attrs, q, component_mod);
                quote! {
                    #name: data.#name.iter().map(|data| {
                        #conversion
                    }).collect(),
                }
            }
            _ => quote! {
                #name: data.#name,
            },
        },
        _ => quote! {
            #name: data.#name,
        },
    }
}

/// Generate a conversion expression from CXX shared struct to a Rust model struct.
/// `data` is assumed to be in scope as the CXX shared struct value.
/// `component_mod` qualifies the struct type (e.g., `icrate::engine`).
fn emit_struct_conversion(
    type_path: &str,
    attrs: &[AttributeDef],
    q: &syn::Path,
    component_mod: &syn::Path,
) -> TokenStream {
    let type_ident: syn::Ident = syn::parse_str(type_path).unwrap();
    let field_conversions: Vec<TokenStream> = attrs
        .iter()
        .map(|a| emit_field_conversion_tokens(a, q, component_mod))
        .collect();
    quote! {
        #component_mod::#type_ident {
            #(#field_conversions)*
        }
    }
}

/// Generate a CXX bridge for an entity with events.
fn emit_entity_bridge(entity: &quent_model::EntityDef, options: &CxxOptions) -> GeneratedFile {
    let entity_name = &entity.name;
    let safe_name = cxx_safe_name(entity_name);
    let ns = format!("{}::{}", options.namespace, safe_name);
    let pascal_name = to_pascal_case(entity_name);
    let observer_name_str = format!("{pascal_name}Observer");
    let observer_name = format_ident!("{}", observer_name_str);
    let q = quent_path(options);
    let icrate: syn::Path = syn::parse_str(&options.instrumentation_crate).unwrap();
    let entity_mod = format_ident!("{}", entity_name);
    let component_mod: syn::Path = syn::parse_str(&format!(
        "{}::{}",
        options.instrumentation_crate, entity_name
    ))
    .unwrap();
    let event_type: syn::Type = syn::parse_str(&options.event_type()).unwrap();
    let include_path = format!("{}/{}/uuid.rs.h", options.crate_name, options.bridge_path);

    // Derive the entity event enum name: e.g., "Job" -> "JobEvent"
    let entity_event_enum = format_ident!("{}Event", pascal_name);

    // Strings for ffi module (CXX-specific syntax)
    let mut shared_structs_str = String::new();
    let mut extern_rust_body = String::new();
    extern_rust_body.push_str(&format!("        type {observer_name_str};\n\n"));
    extern_rust_body.push_str(&format!(
        "        fn create_observer() -> Box<{observer_name_str}>;\n"
    ));

    // Token streams for impl code (standard Rust)
    let mut observer_impl_methods: Vec<TokenStream> = Vec::new();

    for event in &entity.events {
        let event_method = format_ident!("{}", event.name);
        let event_pascal_str = to_pascal_case(&event.name);
        let event_pascal = format_ident!("{}", event_pascal_str);

        if event.attributes.is_empty() {
            // Unit event -- method takes only id
            extern_rust_body.push_str(&format!("        fn {}(&self, id: UUID);\n", event.name,));
            observer_impl_methods.push(quote! {
                pub fn #event_method(&self, id: ffi::UUID) {
                    let model_event = #icrate::#entity_mod::#event_pascal;
                    self.tx.send(#q::Event::new_now(
                        #q::uuid::Uuid::from(id),
                        #icrate::#entity_mod::#entity_event_enum::from(model_event).into(),
                    ));
                }
            });
        } else {
            // Struct event -- generate shared struct and conversion
            let (fields_str, nested_structs) =
                generate_cxx_struct_fields(&event.attributes, &event_pascal_str);
            shared_structs_str.push_str(&nested_structs);
            shared_structs_str.push_str(&format!(
                "    #[derive(Debug)]\n    pub struct {event_pascal_str} {{\n{fields_str}    }}\n\n"
            ));

            extern_rust_body.push_str(&format!(
                "        fn {}(&self, id: UUID, data: {event_pascal_str});\n",
                event.name,
            ));

            let field_conversions: Vec<TokenStream> = event
                .attributes
                .iter()
                .map(|a| emit_field_conversion_tokens(a, &q, &component_mod))
                .collect();

            observer_impl_methods.push(quote! {
                pub fn #event_method(&self, id: ffi::UUID, data: ffi::#event_pascal) {
                    let model_event = #icrate::#entity_mod::#event_pascal {
                        #(#field_conversions)*
                    };
                    self.tx.send(#q::Event::new_now(
                        #q::uuid::Uuid::from(id),
                        #icrate::#entity_mod::#entity_event_enum::from(model_event).into(),
                    ));
                }
            });
        }
    }

    let entity_uses_custom_attrs = entity
        .events
        .iter()
        .any(|ev| attrs_use_custom_attributes(&ev.attributes));
    let ffi_module = build_ffi_module_string(
        &ns,
        &include_path,
        &shared_structs_str,
        &extern_rust_body,
        entity_uses_custom_attrs,
    );

    // Build impl code via quote! + prettyplease
    let impl_tokens = quote! {
        pub struct #observer_name {
            tx: #q::EventSender<#event_type>,
        }

        impl #observer_name {
            #(#observer_impl_methods)*
        }

        pub fn create_observer() -> Box<#observer_name> {
            Box::new(#observer_name {
                tx: super::context::global_sender(),
            })
        }
    };
    let impl_code = pretty_print(impl_tokens);

    GeneratedFile {
        name: format!("{entity_name}.rs"),
        content: format!("{ffi_module}\n{impl_code}"),
    }
}

/// Generate tokens for converting FFI struct fields to a model state struct.
/// `component_mod` is the path prefix for model types (e.g., `icrate::query`).
fn emit_state_conversion_tokens(
    state: &StateDef,
    component_mod: &syn::Path,
    q: &syn::Path,
) -> TokenStream {
    let state_pascal = format_ident!("{}", to_pascal_case(&state.name));

    let attr_fields: Vec<TokenStream> = state
        .attributes
        .iter()
        .map(|a| emit_field_conversion_tokens(a, q, component_mod))
        .collect();

    // Generate type aliases for capacity types (associated type projection
    // cannot be used directly in struct literal position on stable Rust).
    let capacity_aliases: Vec<TokenStream> = state
        .usages
        .iter()
        .map(|usage| {
            let alias = format_ident!("__{}Capacity", to_pascal_case(&usage.field_name));
            let resource_ty: syn::Type = syn::parse_str(&usage.resource_type_path).unwrap();
            quote! {
                type #alias = <#resource_ty as #q::Resource>::CapacityValue;
            }
        })
        .collect();

    let usage_fields: Vec<TokenStream> = state
        .usages
        .iter()
        .map(|usage| {
            let field_name = format_ident!("{}", usage.field_name);
            let resource_id_field = format_ident!("{}_resource_id", usage.field_name);
            let alias = format_ident!("__{}Capacity", to_pascal_case(&usage.field_name));

            quote! {
                #field_name: #q::Usage {
                    resource_id: #q::Ref::new(#q::uuid::Uuid::from(data.#resource_id_field)),
                    capacity: #alias {},
                },
            }
        })
        .collect();

    quote! {
        #(#capacity_aliases)*
        let state = #component_mod::#state_pascal {
            #(#attr_fields)*
            #(#usage_fields)*
        };
    }
}

/// Generate a CXX bridge for an FSM.
fn emit_fsm_bridge(fsm: &FsmDef, options: &CxxOptions) -> GeneratedFile {
    let fsm_name = &fsm.name;
    let safe_name = cxx_safe_name(fsm_name);
    let ns = format!("{}::{}", options.namespace, safe_name);
    let pascal_name = to_pascal_case(fsm_name);
    let handle_name_str = format!("{pascal_name}Handle");
    let handle_name = format_ident!("{}", handle_name_str);
    let q = quent_path(options);
    let icrate: syn::Path = syn::parse_str(&options.instrumentation_crate).unwrap();
    let fsm_mod = format_ident!("{}", fsm_name);
    let component_mod: syn::Path =
        syn::parse_str(&format!("{}::{}", options.instrumentation_crate, fsm_name)).unwrap();
    let include_path = format!("{}/{}/uuid.rs.h", options.crate_name, options.bridge_path);

    let model_handle: syn::Type = {
        let s = format!(
            "{}::{}::{}Handle<{}>",
            options.instrumentation_crate,
            fsm_name,
            pascal_name,
            options.event_type(),
        );
        syn::parse_str(&s).unwrap()
    };

    // Determine the entry state (first state in the list, which is the #[entry] state)
    let entry_state = &fsm.states[0];
    let entry_pascal_str = to_pascal_case(&entry_state.name);
    let entry_pascal = format_ident!("{}", entry_pascal_str);
    let entry_name = format_ident!("{}", entry_state.name);
    let has_entry_data = !entry_state.attributes.is_empty() || !entry_state.usages.is_empty();

    // Build ffi module shared structs as string
    let mut shared_structs_str = String::new();
    for state in &fsm.states {
        if state.attributes.is_empty() && state.usages.is_empty() {
            continue;
        }
        let state_pascal = to_pascal_case(&state.name);
        let (attr_fields_str, nested_structs) =
            generate_cxx_struct_fields(&state.attributes, &state_pascal);
        shared_structs_str.push_str(&nested_structs);
        let mut fields_str = attr_fields_str;
        for usage in &state.usages {
            fields_str.push_str(&format!(
                "        pub {}_resource_id: UUID,\n",
                usage.field_name
            ));
        }
        shared_structs_str.push_str(&format!(
            "    #[derive(Debug)]\n    pub struct {state_pascal} {{\n{fields_str}    }}\n\n"
        ));
    }

    // Build extern "Rust" body as string
    let mut extern_rust_body = String::new();
    extern_rust_body.push_str(&format!("        type {handle_name_str};\n\n"));

    // Factory method
    if has_entry_data {
        extern_rust_body.push_str(&format!(
            "        fn create(data: {entry_pascal_str}) -> Box<{handle_name_str}>;\n"
        ));
    } else {
        extern_rust_body.push_str(&format!("        fn create() -> Box<{handle_name_str}>;\n"));
    }

    // Transition methods
    for state in &fsm.states {
        let state_pascal = to_pascal_case(&state.name);
        if state.attributes.is_empty() && state.usages.is_empty() {
            extern_rust_body.push_str(&format!("        fn {}(&mut self);\n", state.name));
        } else {
            extern_rust_body.push_str(&format!(
                "        fn {}(&mut self, data: {state_pascal});\n",
                state.name,
            ));
        }
    }
    extern_rust_body.push_str("        fn exit(&mut self);\n");

    let ffi_module = build_ffi_module_string(
        &ns,
        &include_path,
        &shared_structs_str,
        &extern_rust_body,
        false,
    );

    // Build impl code via quote! + prettyplease
    let impl_transition_methods: Vec<TokenStream> = fsm
        .states
        .iter()
        .map(|state| {
            let method_name = format_ident!("{}", state.name);
            let state_pascal_ident = format_ident!("{}", to_pascal_case(&state.name));
            if state.attributes.is_empty() && state.usages.is_empty() {
                quote! {
                    pub fn #method_name(&mut self) {
                        let state = #icrate::#fsm_mod::#state_pascal_ident;
                        self.inner.transition(state);
                    }
                }
            } else {
                let conversion = emit_state_conversion_tokens(state, &component_mod, &q);
                quote! {
                    pub fn #method_name(&mut self, data: ffi::#state_pascal_ident) {
                        #conversion
                        self.inner.transition(state);
                    }
                }
            }
        })
        .collect();

    let observer_name = format_ident!("{}Observer", pascal_name);
    let factory_fn = if has_entry_data {
        let conversion = emit_state_conversion_tokens(entry_state, &component_mod, &q);
        quote! {
            pub fn create(data: ffi::#entry_pascal) -> Box<#handle_name> {
                #conversion
                let obs = #icrate::#fsm_mod::#observer_name::new(&super::context::global_sender());
                let id = #q::uuid::Uuid::now_v7();
                Box::new(#handle_name {
                    inner: obs.#entry_name(id, state),
                })
            }
        }
    } else {
        quote! {
            pub fn create() -> Box<#handle_name> {
                let state = #icrate::#fsm_mod::#entry_pascal;
                let obs = #icrate::#fsm_mod::#observer_name::new(&super::context::global_sender());
                let id = #q::uuid::Uuid::now_v7();
                Box::new(#handle_name {
                    inner: obs.#entry_name(id, state),
                })
            }
        }
    };

    let impl_tokens = quote! {
        pub struct #handle_name {
            inner: #model_handle,
        }

        impl #handle_name {
            #(#impl_transition_methods)*

            pub fn exit(&mut self) {
                self.inner.exit();
            }
        }

        #factory_fn
    };
    let impl_code = pretty_print(impl_tokens);

    GeneratedFile {
        name: format!("{fsm_name}.rs"),
        content: format!("{ffi_module}\n{impl_code}"),
    }
}

/// Generate the lib.rs that includes all modules.
fn emit_lib_rs(model: &ModelBuilder, _options: &CxxOptions) -> GeneratedFile {
    let mut mod_items: Vec<TokenStream> = Vec::new();
    mod_items.push(quote! { pub mod uuid; });
    mod_items.push(quote! { pub mod context; });
    for entity in &model.entities {
        let name = format_ident!("{}", entity.name);
        mod_items.push(quote! { pub mod #name; });
    }
    for fsm in &model.fsms {
        let name = format_ident!("{}", fsm.name);
        mod_items.push(quote! { pub mod #name; });
    }

    let tokens = quote! {
        #(#mod_items)*
    };

    GeneratedFile {
        name: "lib.rs".to_string(),
        content: pretty_print(tokens),
    }
}
