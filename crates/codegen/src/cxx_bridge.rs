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
use quote::{ToTokens, format_ident, quote};

use quent_model::{AttributeDef, FsmDef, ModelBuilder, StateDef, ValueType};

use crate::{CxxOptions, GeneratedFile};

/// Map a Quent `ValueType` to a CXX-compatible Rust type string.
fn value_type_to_cxx(ty: &ValueType) -> &'static str {
    match ty {
        ValueType::Bool => "bool",
        ValueType::Uuid => "UUID",
        ValueType::String => "String",
        ValueType::U8 => "u8",
        ValueType::U16 => "u16",
        ValueType::U32 => "u32",
        ValueType::U64 => "u64",
        ValueType::I8 => "i8",
        ValueType::I16 => "i16",
        ValueType::I32 => "i32",
        ValueType::I64 => "i64",
        ValueType::F32 => "f32",
        ValueType::F64 => "f64",
        ValueType::Ref(_) => "UUID",
        ValueType::List(_) | ValueType::Struct(_) => "String", // fallback: serialize as JSON string
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
    files.push(emit_uuid_bridge(options));

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

/// Generate the UUID shared type bridge.
fn emit_uuid_bridge(_options: &CxxOptions) -> GeneratedFile {
    let tokens = quote! {
        #[cxx::bridge(namespace = "uuid")]
        pub mod ffi {
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
            }
        }

        fn uuid_now_v7() -> ffi::UUID {
            let id = uuid::Uuid::now_v7();
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

        impl From<ffi::UUID> for uuid::Uuid {
            fn from(u: ffi::UUID) -> Self {
                uuid::Uuid::from_u64_pair(u.high_bits, u.low_bits)
            }
        }

        impl From<uuid::Uuid> for ffi::UUID {
            fn from(u: uuid::Uuid) -> Self {
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
    let event_type: syn::Type = syn::parse_str(&options.event_type).unwrap();

    let tokens = quote! {
        use std::sync::OnceLock;

        #[cxx::bridge(namespace = #ns)]
        pub mod ffi {
            extern "Rust" {
                type Context;
                fn create_context(exporter: String, output_dir: String) -> Box<Context>;
            }
        }

        /// Global event sender, initialized by `create_context`.
        static SENDER: OnceLock<quent_model::EventSender<#event_type>> = OnceLock::new();

        pub struct Context {
            _inner: quent_instrumentation::Context<#event_type>,
        }

        pub fn global_sender() -> quent_model::EventSender<#event_type> {
            SENDER
                .get()
                .expect("create_context must be called first")
                .clone()
        }

        pub fn create_context(exporter: String, output_dir: String) -> Box<Context> {
            let opts = match exporter.as_str() {
                "ndjson" => Some(quent_exporter::ExporterOptions::Ndjson(
                    quent_exporter::NdjsonExporterOptions {
                        output_dir: output_dir.into(),
                    },
                )),
                _ => None,
            };
            let inner =
                quent_instrumentation::Context::try_new(opts, uuid::Uuid::now_v7()).unwrap();
            let _ = SENDER.set(inner.events_sender());
            Box::new(Context { _inner: inner })
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
) -> String {
    format!(
        r#"#[cxx::bridge(namespace = "{ns}")]
pub mod ffi {{
    #[namespace = "uuid"]
    unsafe extern "C++" {{
        include!("{include_path}");
        type UUID = crate::bridge::uuid::ffi::UUID;
    }}

{shared_structs}    extern "Rust" {{
{extern_rust_body}    }}
}}
"#
    )
}

/// Generate a field conversion expression for an attribute: `name: <conversion>(data.name)`.
fn emit_field_conversion_tokens(attr: &AttributeDef) -> TokenStream {
    let name = format_ident!("{}", attr.name);
    match &attr.value_type {
        ValueType::Uuid => quote! {
            #name: uuid::Uuid::from(data.#name),
        },
        ValueType::Ref(_) => quote! {
            #name: quent_model::Ref::new(uuid::Uuid::from(data.#name)),
        },
        _ => quote! {
            #name: data.#name,
        },
    }
}

/// Generate a CXX bridge for an entity with events.
fn emit_entity_bridge(
    entity: &quent_model::EntityDef,
    options: &CxxOptions,
) -> GeneratedFile {
    let entity_name = &entity.name;
    let ns = format!("{}::{}", options.namespace, entity_name);
    let pascal_name = to_pascal_case(entity_name);
    let observer_name_str = format!("{pascal_name}Observer");
    let observer_name = format_ident!("{}", observer_name_str);
    let model_crate: syn::Path = syn::parse_str(&options.model_crate).unwrap();
    let event_type: syn::Type = syn::parse_str(&options.event_type).unwrap();
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
            extern_rust_body.push_str(&format!(
                "        fn {}(&self, id: UUID);\n",
                event.name,
            ));
            observer_impl_methods.push(quote! {
                pub fn #event_method(&self, id: ffi::UUID) {
                    let model_event = #model_crate::#event_pascal;
                    self.tx.send(quent_events::Event::new_now(
                        uuid::Uuid::from(id),
                        #model_crate::#entity_event_enum::from(model_event).into(),
                    ));
                }
            });
        } else {
            // Struct event -- generate shared struct and conversion
            let mut fields_str = String::new();
            for attr in &event.attributes {
                let cxx_type = value_type_to_cxx(&attr.value_type);
                fields_str.push_str(&format!("        pub {}: {},\n", attr.name, cxx_type));
            }

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
                .map(emit_field_conversion_tokens)
                .collect();

            observer_impl_methods.push(quote! {
                pub fn #event_method(&self, id: ffi::UUID, data: ffi::#event_pascal) {
                    let model_event = #model_crate::#event_pascal {
                        #(#field_conversions)*
                    };
                    self.tx.send(quent_events::Event::new_now(
                        uuid::Uuid::from(id),
                        #model_crate::#entity_event_enum::from(model_event).into(),
                    ));
                }
            });
        }
    }

    // Build ffi module as string
    let ffi_module =
        build_ffi_module_string(&ns, &include_path, &shared_structs_str, &extern_rust_body);

    // Build impl code via quote! + prettyplease
    let impl_tokens = quote! {
        pub struct #observer_name {
            tx: quent_model::EventSender<#event_type>,
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
fn emit_state_conversion_tokens(state: &StateDef, model_crate: &syn::Path) -> TokenStream {
    let state_pascal = format_ident!("{}", to_pascal_case(&state.name));

    let attr_fields: Vec<TokenStream> = state
        .attributes
        .iter()
        .map(emit_field_conversion_tokens)
        .collect();

    let usage_fields: Vec<TokenStream> = state
        .usages
        .iter()
        .map(|usage| {
            let field_name = format_ident!("{}", usage.field_name);
            let resource_id_field = format_ident!("{}_resource_id", usage.field_name);
            let capacity_type: syn::Type = match usage.resource_name.as_str() {
                "processor" => syn::parse_str("quent_stdlib::ProcessorOperating").unwrap(),
                "memory" => syn::parse_str("quent_stdlib::MemoryOperating").unwrap(),
                "channel" => syn::parse_str("quent_stdlib::ChannelOperating").unwrap(),
                other => {
                    let pascal = to_pascal_case(other);
                    let path = format!("{}::{}Operating", model_crate.to_token_stream(), pascal);
                    syn::parse_str(&path).unwrap()
                }
            };

            if usage.capacities.is_empty() {
                quote! {
                    #field_name: quent_model::Usage {
                        resource_id: quent_model::Ref::new(uuid::Uuid::from(data.#resource_id_field)),
                        capacity: #capacity_type {},
                    },
                }
            } else {
                let cap_fields: Vec<TokenStream> = usage
                    .capacities
                    .iter()
                    .map(|cap| {
                        let cap_name = format_ident!("{}", cap.name);
                        let data_field = format_ident!("{}_{}", usage.field_name, cap.name);
                        quote! {
                            #cap_name: quent_model::Capacity::new(data.#data_field),
                        }
                    })
                    .collect();

                quote! {
                    #field_name: quent_model::Usage {
                        resource_id: quent_model::Ref::new(uuid::Uuid::from(data.#resource_id_field)),
                        capacity: #capacity_type {
                            #(#cap_fields)*
                        },
                    },
                }
            }
        })
        .collect();

    quote! {
        let state = #model_crate::#state_pascal {
            #(#attr_fields)*
            #(#usage_fields)*
        };
    }
}

/// Generate a CXX bridge for an FSM.
fn emit_fsm_bridge(fsm: &FsmDef, options: &CxxOptions) -> GeneratedFile {
    let fsm_name = &fsm.name;
    let ns = format!("{}::{}", options.namespace, fsm_name);
    let pascal_name = to_pascal_case(fsm_name);
    let handle_name_str = format!("{pascal_name}Handle");
    let handle_name = format_ident!("{}", handle_name_str);
    let model_crate: syn::Path = syn::parse_str(&options.model_crate).unwrap();
    let include_path = format!("{}/{}/uuid.rs.h", options.crate_name, options.bridge_path);

    let model_handle: syn::Type = {
        let s = format!(
            "{}::{}Handle<{}>",
            options.model_crate, pascal_name, options.event_type,
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
        let mut fields_str = String::new();
        for attr in &state.attributes {
            let cxx_type = value_type_to_cxx(&attr.value_type);
            fields_str.push_str(&format!("        pub {}: {},\n", attr.name, cxx_type));
        }
        for usage in &state.usages {
            fields_str.push_str(&format!(
                "        pub {}_resource_id: UUID,\n",
                usage.field_name
            ));
            for cap in &usage.capacities {
                let cxx_type = value_type_to_cxx(&cap.value_type);
                fields_str.push_str(&format!(
                    "        pub {}_{}: {},\n",
                    usage.field_name, cap.name, cxx_type
                ));
            }
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
        extern_rust_body.push_str(&format!(
            "        fn create() -> Box<{handle_name_str}>;\n"
        ));
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

    let ffi_module =
        build_ffi_module_string(&ns, &include_path, &shared_structs_str, &extern_rust_body);

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
                        let state = #model_crate::#state_pascal_ident;
                        self.inner.transition(state);
                    }
                }
            } else {
                let conversion = emit_state_conversion_tokens(state, &model_crate);
                quote! {
                    pub fn #method_name(&mut self, data: ffi::#state_pascal_ident) {
                        #conversion
                        self.inner.transition(state);
                    }
                }
            }
        })
        .collect();

    let factory_fn = if has_entry_data {
        let conversion = emit_state_conversion_tokens(entry_state, &model_crate);
        quote! {
            pub fn create(data: ffi::#entry_pascal) -> Box<#handle_name> {
                #conversion
                Box::new(#handle_name {
                    inner: #model_crate::#handle_name::#entry_name(
                        &super::context::global_sender(),
                        state,
                    ),
                })
            }
        }
    } else {
        quote! {
            pub fn create() -> Box<#handle_name> {
                let state = #model_crate::#entry_pascal;
                Box::new(#handle_name {
                    inner: #model_crate::#handle_name::#entry_name(
                        &super::context::global_sender(),
                        state,
                    ),
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
