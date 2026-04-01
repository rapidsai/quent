// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! CXX bridge code generator.
//!
//! Generates Rust `#[cxx::bridge]` modules from model definitions. The output
//! is Rust source code that CXX compiles into C++ headers.

use quent_model::{
    AttributeDef, EntityDef, EntityEventDef, FsmDef, ModelBuilder, StateDef, ValueType,
};

use crate::{CxxOptions, GeneratedFile};

/// Map a Quent ValueType to a CXX-compatible Rust type string.
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
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut result = first.to_uppercase().to_string();
                    result.extend(chars);
                    result
                }
            }
        })
        .collect()
}

/// Generate CXX bridge files for all model components.
pub fn emit(model: &ModelBuilder, options: &CxxOptions) -> Vec<GeneratedFile> {
    let mut files = Vec::new();

    // Generate UUID bridge (shared type used by all bridges)
    files.push(emit_uuid_bridge(options));

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
fn emit_uuid_bridge(options: &CxxOptions) -> GeneratedFile {
    let ns = &options.namespace;
    GeneratedFile {
        name: "uuid.rs".to_string(),
        content: format!(
            r#"#[cxx::bridge(namespace = "uuid")]
pub mod ffi {{
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct UUID {{
        pub high_bits: u64,
        pub low_bits: u64,
    }}

    extern "Rust" {{
        #[cxx_name = "now_v7"]
        fn uuid_now_v7() -> UUID;

        #[cxx_name = "new_nil"]
        fn uuid_new_nil() -> UUID;
    }}
}}

fn uuid_now_v7() -> ffi::UUID {{
    let id = uuid::Uuid::now_v7();
    let (high, low) = id.as_u64_pair();
    ffi::UUID {{ high_bits: high, low_bits: low }}
}}

fn uuid_new_nil() -> ffi::UUID {{
    ffi::UUID {{ high_bits: 0, low_bits: 0 }}
}}

impl From<ffi::UUID> for uuid::Uuid {{
    fn from(u: ffi::UUID) -> Self {{
        uuid::Uuid::from_u64_pair(u.high_bits, u.low_bits)
    }}
}}

impl From<uuid::Uuid> for ffi::UUID {{
    fn from(u: uuid::Uuid) -> Self {{
        let (high, low) = u.as_u64_pair();
        ffi::UUID {{ high_bits: high, low_bits: low }}
    }}
}}
"#
        ),
    }
}

/// Generate a CXX bridge for an entity with events.
fn emit_entity_bridge(entity: &EntityDef, options: &CxxOptions) -> GeneratedFile {
    let ns = &options.namespace;
    let entity_name = &entity.name;
    let pascal_name = to_pascal_case(entity_name);
    let observer_name = format!("{pascal_name}Observer");

    let mut shared_structs = String::new();
    let mut observer_methods = String::new();
    let mut observer_impl_methods = String::new();

    for event in &entity.events {
        let event_pascal = to_pascal_case(&event.name);

        // Shared struct for the event
        if event.attributes.is_empty() {
            // Unit event — no shared struct needed, method takes no data arg
            observer_methods.push_str(&format!(
                "        fn {name}(&self, id: UUID);\n",
                name = event.name,
            ));
            observer_impl_methods.push_str(&format!(
                r#"    pub fn {name}(&self, id: ffi::UUID) {{
        // Emit {event_pascal} event
        let _ = id;
        todo!("emit {entity_name}::{event_pascal} event")
    }}
"#,
                name = event.name,
                event_pascal = event_pascal,
                entity_name = entity_name,
            ));
        } else {
            // Struct event
            let mut fields = String::new();
            for attr in &event.attributes {
                let cxx_type = value_type_to_cxx(&attr.value_type);
                fields.push_str(&format!("        pub {}: {},\n", attr.name, cxx_type));
            }

            shared_structs.push_str(&format!(
                r#"    #[derive(Debug)]
    pub struct {event_pascal} {{
{fields}    }}

"#
            ));

            observer_methods.push_str(&format!(
                "        fn {name}(&self, id: UUID, data: {event_pascal});\n",
                name = event.name,
                event_pascal = event_pascal,
            ));

            observer_impl_methods.push_str(&format!(
                r#"    pub fn {name}(&self, id: ffi::UUID, data: ffi::{event_pascal}) {{
        // Emit {entity_name}::{event_pascal} event
        let _ = (id, data);
        todo!("emit {entity_name}::{event_pascal} event")
    }}
"#,
                name = event.name,
                event_pascal = event_pascal,
                entity_name = entity_name,
            ));
        }
    }

    let content = format!(
        r#"#[cxx::bridge(namespace = "{ns}::{entity_name}")]
pub mod ffi {{
    // Import UUID from the uuid bridge
    #[namespace = "uuid"]
    extern "C++" {{
        include!("uuid.hpp");
        type UUID = crate::uuid::ffi::UUID;
    }}

{shared_structs}    extern "Rust" {{
        type {observer_name};

        fn create_observer(/* context */) -> Box<{observer_name}>;
{observer_methods}    }}
}}

pub struct {observer_name} {{
    // tx: EventSender<AppEvent>,
}}

impl {observer_name} {{
{observer_impl_methods}}}

pub fn create_observer() -> Box<{observer_name}> {{
    Box::new({observer_name} {{}})
}}
"#
    );

    GeneratedFile {
        name: format!("{entity_name}.rs"),
        content,
    }
}

/// Generate a CXX bridge for an FSM.
fn emit_fsm_bridge(fsm: &FsmDef, options: &CxxOptions) -> GeneratedFile {
    let ns = &options.namespace;
    let fsm_name = &fsm.name;
    let pascal_name = to_pascal_case(fsm_name);
    let handle_name = format!("{pascal_name}Handle");

    let mut shared_structs = String::new();
    let mut handle_methods = String::new();

    // Generate shared struct per state (for transition data)
    for state in &fsm.states {
        let state_pascal = to_pascal_case(&state.name);

        if state.attributes.is_empty() && state.usages.is_empty() {
            // Unit state — no shared struct
        } else {
            let mut fields = String::new();
            for attr in &state.attributes {
                let cxx_type = value_type_to_cxx(&attr.value_type);
                fields.push_str(&format!("        pub {}: {},\n", attr.name, cxx_type));
            }
            for usage in &state.usages {
                // Each usage contributes a resource_id field
                fields.push_str(&format!(
                    "        pub {}_resource_id: UUID,\n",
                    usage.field_name
                ));
                for cap in &usage.capacities {
                    let cxx_type = value_type_to_cxx(&cap.value_type);
                    fields.push_str(&format!(
                        "        pub {}_{}: {},\n",
                        usage.field_name, cap.name, cxx_type
                    ));
                }
            }

            shared_structs.push_str(&format!(
                r#"    #[derive(Debug)]
    pub struct {state_pascal} {{
{fields}    }}

"#
            ));
        }
    }

    // Handle methods: new, one per state transition, exit
    handle_methods.push_str(&format!(
        "        fn create_{fsm_name}(/* context, initial_state */) -> Box<{handle_name}>;\n"
    ));
    for state in &fsm.states {
        let state_pascal = to_pascal_case(&state.name);
        if state.attributes.is_empty() && state.usages.is_empty() {
            handle_methods.push_str(&format!(
                "        fn {name}(&mut self);\n",
                name = state.name,
            ));
        } else {
            handle_methods.push_str(&format!(
                "        fn {name}(&mut self, data: {state_pascal});\n",
                name = state.name,
                state_pascal = state_pascal,
            ));
        }
    }
    handle_methods.push_str(&format!(
        "        fn exit(&mut self);\n"
    ));

    let content = format!(
        r#"#[cxx::bridge(namespace = "{ns}::{fsm_name}")]
pub mod ffi {{
    // Import UUID from the uuid bridge
    #[namespace = "uuid"]
    extern "C++" {{
        include!("uuid.hpp");
        type UUID = crate::uuid::ffi::UUID;
    }}

{shared_structs}    extern "Rust" {{
        type {handle_name};

{handle_methods}    }}
}}

pub struct {handle_name} {{
    // inner: quent_model::TaskHandle<AppEvent>,
}}

// TODO: implement handle methods
"#
    );

    GeneratedFile {
        name: format!("{fsm_name}.rs"),
        content,
    }
}

/// Generate the lib.rs that includes all modules.
fn emit_lib_rs(model: &ModelBuilder, _options: &CxxOptions) -> GeneratedFile {
    let mut mods = String::new();
    mods.push_str("pub mod uuid;\n");
    for entity in &model.entities {
        mods.push_str(&format!("pub mod {};\n", entity.name));
    }
    for fsm in &model.fsms {
        mods.push_str(&format!("pub mod {};\n", fsm.name));
    }

    GeneratedFile {
        name: "lib.rs".to_string(),
        content: mods,
    }
}
