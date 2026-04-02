// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! CXX bridge code generator.
//!
//! Generates Rust `#[cxx::bridge]` modules from model definitions. The output
//! is Rust source code that CXX compiles into C++ headers.
//!
//! TODO: The code generation currently uses `format!()` string concatenation.
//! Using `quote!` (as a runtime dependency, not proc-macro) would be more
//! robust, providing automatic identifier hygiene and compile-time token
//! validation. However, `format!()` works correctly for the current scope and
//! the generated code is validated by `cxx_build` and `syn::parse_file()` in
//! tests.

use quent_model::{AttributeDef, FsmDef, ModelBuilder, StateDef, ValueType};

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
    GeneratedFile {
        name: "uuid.rs".to_string(),
        content: r#"#[cxx::bridge(namespace = "uuid")]
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
    ffi::UUID { high_bits: high, low_bits: low }
}

fn uuid_new_nil() -> ffi::UUID {
    ffi::UUID { high_bits: 0, low_bits: 0 }
}

impl From<ffi::UUID> for uuid::Uuid {
    fn from(u: ffi::UUID) -> Self {
        uuid::Uuid::from_u64_pair(u.high_bits, u.low_bits)
    }
}

impl From<uuid::Uuid> for ffi::UUID {
    fn from(u: uuid::Uuid) -> Self {
        let (high, low) = u.as_u64_pair();
        ffi::UUID { high_bits: high, low_bits: low }
    }
}
"#
        .to_string(),
    }
}

/// Generate the context bridge module.
///
/// The context is created once and stores the event sender in a global static.
/// This avoids the need to share opaque Rust types across CXX bridge modules.
fn emit_context_bridge(options: &CxxOptions) -> GeneratedFile {
    let ns = &options.namespace;
    let event_type = &options.event_type;

    GeneratedFile {
        name: "context.rs".to_string(),
        content: format!(
            r#"use std::sync::OnceLock;

#[cxx::bridge(namespace = "{ns}")]
pub mod ffi {{
    extern "Rust" {{
        type Context;
        fn create_context(exporter: String, output_dir: String) -> Box<Context>;
    }}
}}

/// Global event sender, initialized by `create_context`.
static SENDER: OnceLock<quent_model::EventSender<{event_type}>> = OnceLock::new();

pub struct Context {{
    _inner: quent_instrumentation::Context<{event_type}>,
}}

pub fn global_sender() -> quent_model::EventSender<{event_type}> {{
    SENDER.get().expect("create_context must be called first").clone()
}}

pub fn create_context(exporter: String, output_dir: String) -> Box<Context> {{
    let opts = match exporter.as_str() {{
        "ndjson" => Some(quent_exporter::ExporterOptions::Ndjson(
            quent_exporter::NdjsonExporterOptions {{
                output_dir: output_dir.into(),
            }},
        )),
        _ => None,
    }};
    let inner = quent_instrumentation::Context::try_new(opts, uuid::Uuid::now_v7()).unwrap();
    let _ = SENDER.set(inner.events_sender());
    Box::new(Context {{ _inner: inner }})
}}
"#
        ),
    }
}

/// Generate a CXX bridge for an entity with events.
fn emit_entity_bridge(
    entity: &quent_model::EntityDef,
    options: &CxxOptions,
) -> GeneratedFile {
    let ns = &options.namespace;
    let entity_name = &entity.name;
    let pascal_name = to_pascal_case(entity_name);
    let observer_name = format!("{pascal_name}Observer");
    let model_crate = &options.model_crate;
    let event_type = &options.event_type;
    let crate_name = &options.crate_name;
    let bridge_path = &options.bridge_path;

    // Derive the entity event enum name: e.g., "Job" -> "JobEvent"
    let entity_event_enum = format!("{pascal_name}Event");

    let mut shared_structs = String::new();
    let mut observer_methods = String::new();
    let mut observer_impl_methods = String::new();

    for event in &entity.events {
        let event_pascal = to_pascal_case(&event.name);

        if event.attributes.is_empty() {
            // Unit event — method takes only id
            observer_methods.push_str(&format!(
                "        fn {name}(&self, id: UUID);\n",
                name = event.name,
            ));
            observer_impl_methods.push_str(&format!(
                r#"    pub fn {name}(&self, id: ffi::UUID) {{
        let model_event = {model_crate}::{event_pascal};
        self.tx.send(quent_events::Event::new_now(
            uuid::Uuid::from(id),
            {model_crate}::{entity_event_enum}::from(model_event).into(),
        ));
    }}
"#,
                name = event.name,
            ));
        } else {
            // Struct event — generate shared struct and conversion
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

            // Generate field-by-field conversion
            let mut field_conversions = String::new();
            for attr in &event.attributes {
                field_conversions.push_str(&emit_field_conversion(attr));
                field_conversions.push('\n');
            }

            observer_impl_methods.push_str(&format!(
                r#"    pub fn {name}(&self, id: ffi::UUID, data: ffi::{event_pascal}) {{
        let model_event = {model_crate}::{event_pascal} {{
{field_conversions}        }};
        self.tx.send(quent_events::Event::new_now(
            uuid::Uuid::from(id),
            {model_crate}::{entity_event_enum}::from(model_event).into(),
        ));
    }}
"#,
                name = event.name,
            ));
        }
    }

    let content = format!(
        r#"#[cxx::bridge(namespace = "{ns}::{entity_name}")]
pub mod ffi {{
    #[namespace = "uuid"]
    unsafe extern "C++" {{
        include!("{crate_name}/{bridge_path}/uuid.rs.h");
        type UUID = crate::bridge::uuid::ffi::UUID;
    }}

{shared_structs}    extern "Rust" {{
        type {observer_name};

        fn create_observer() -> Box<{observer_name}>;
{observer_methods}    }}
}}

pub struct {observer_name} {{
    tx: quent_model::EventSender<{event_type}>,
}}

impl {observer_name} {{
{observer_impl_methods}}}

pub fn create_observer() -> Box<{observer_name}> {{
    Box::new({observer_name} {{
        tx: super::context::global_sender(),
    }})
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
    let model_crate = &options.model_crate;
    let event_type = &options.event_type;
    let crate_name = &options.crate_name;
    let bridge_path = &options.bridge_path;

    let mut shared_structs = String::new();
    let mut handle_methods = String::new();
    let mut handle_impl_methods = String::new();

    // Determine the entry state (first state in the list, which is the #[entry] state)
    let entry_state = &fsm.states[0];
    let entry_pascal = to_pascal_case(&entry_state.name);

    // Generate shared struct per state (for transition data)
    for state in &fsm.states {
        let state_pascal = to_pascal_case(&state.name);

        if state.attributes.is_empty() && state.usages.is_empty() {
            // Unit state — no shared struct needed
        } else {
            let mut fields = String::new();
            for attr in &state.attributes {
                let cxx_type = value_type_to_cxx(&attr.value_type);
                fields.push_str(&format!("        pub {}: {},\n", attr.name, cxx_type));
            }
            for usage in &state.usages {
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

    // Factory function: create with initial (entry) state
    let has_entry_data = !entry_state.attributes.is_empty() || !entry_state.usages.is_empty();
    if has_entry_data {
        handle_methods.push_str(&format!(
            "        fn create(data: {entry_pascal}) -> Box<{handle_name}>;\n"
        ));
    } else {
        handle_methods.push_str(&format!(
            "        fn create() -> Box<{handle_name}>;\n"
        ));
    }

    // Transition methods per state
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
    handle_methods.push_str("        fn exit(&mut self);\n");

    // Generate impl methods for each state transition
    for state in &fsm.states {
        let state_pascal = to_pascal_case(&state.name);
        if state.attributes.is_empty() && state.usages.is_empty() {
            handle_impl_methods.push_str(&format!(
                r#"    pub fn {name}(&mut self) {{
        let state = {model_crate}::{state_pascal};
        self.inner.transition(state);
    }}
"#,
                name = state.name,
            ));
        } else {
            let conversion = emit_state_conversion(state, model_crate);
            handle_impl_methods.push_str(&format!(
                r#"    pub fn {name}(&mut self, data: ffi::{state_pascal}) {{
{conversion}        self.inner.transition(state);
    }}
"#,
                name = state.name,
            ));
        }
    }

    // Exit method
    handle_impl_methods.push_str(
        r#"    pub fn exit(&mut self) {
        self.inner.exit();
    }
"#,
    );

    // Factory function implementation
    let factory_fn = if has_entry_data {
        let conversion = emit_state_conversion(entry_state, model_crate);
        format!(
            r#"pub fn create(data: ffi::{entry_pascal}) -> Box<{handle_name}> {{
{conversion}    Box::new({handle_name} {{
        inner: {model_crate}::{pascal_name}Handle::new(&super::context::global_sender(), state),
    }})
}}
"#
        )
    } else {
        format!(
            r#"pub fn create() -> Box<{handle_name}> {{
    let state = {model_crate}::{entry_pascal};
    Box::new({handle_name} {{
        inner: {model_crate}::{pascal_name}Handle::new(&super::context::global_sender(), state),
    }})
}}
"#
        )
    };

    let content = format!(
        r#"#[cxx::bridge(namespace = "{ns}::{fsm_name}")]
pub mod ffi {{
    #[namespace = "uuid"]
    unsafe extern "C++" {{
        include!("{crate_name}/{bridge_path}/uuid.rs.h");
        type UUID = crate::bridge::uuid::ffi::UUID;
    }}

{shared_structs}    extern "Rust" {{
        type {handle_name};

{handle_methods}    }}
}}

pub struct {handle_name} {{
    inner: {model_crate}::{pascal_name}Handle<{event_type}>,
}}

impl {handle_name} {{
{handle_impl_methods}}}

{factory_fn}"#
    );

    GeneratedFile {
        name: format!("{fsm_name}.rs"),
        content,
    }
}

/// Generate a state conversion block from FFI struct fields to model struct.
fn emit_state_conversion(state: &StateDef, model_crate: &str) -> String {
    let state_pascal = to_pascal_case(&state.name);
    let mut lines = String::new();
    lines.push_str(&format!(
        "        let state = {model_crate}::{state_pascal} {{\n"
    ));

    for attr in &state.attributes {
        let name = &attr.name;
        match &attr.value_type {
            ValueType::Uuid => {
                lines.push_str(&format!(
                    "            {name}: uuid::Uuid::from(data.{name}),\n"
                ));
            }
            ValueType::Ref(_) => {
                lines.push_str(&format!(
                    "            {name}: quent_model::Ref::new(uuid::Uuid::from(data.{name})),\n"
                ));
            }
            _ => {
                lines.push_str(&format!("            {name}: data.{name},\n"));
            }
        }
    }

    for usage in &state.usages {
        let field_name = &usage.field_name;
        if usage.capacities.is_empty() {
            // Unit resource usage — capacity is a unit struct from stdlib
            let capacity_type = match usage.resource_name.as_str() {
                "processor" => "quent_stdlib::ProcessorOperating".to_string(),
                "memory" => "quent_stdlib::MemoryOperating".to_string(),
                "channel" => "quent_stdlib::ChannelOperating".to_string(),
                other => {
                    let pascal = to_pascal_case(other);
                    format!("{model_crate}::{pascal}Operating")
                }
            };
            lines.push_str(&format!(
                "            {field_name}: quent_model::Usage {{\n"
            ));
            lines.push_str(&format!(
                "                resource_id: quent_model::Ref::new(uuid::Uuid::from(data.{field_name}_resource_id)),\n"
            ));
            lines.push_str(&format!(
                "                capacity: {capacity_type} {{}},\n"
            ));
            lines.push_str("            },\n");
        } else {
            // Resource with capacity fields — construct the capacity struct
            let capacity_type = match usage.resource_name.as_str() {
                "memory" => "quent_stdlib::MemoryOperating".to_string(),
                "channel" => "quent_stdlib::ChannelOperating".to_string(),
                other => {
                    let pascal = to_pascal_case(other);
                    format!("{model_crate}::{pascal}Operating")
                }
            };
            lines.push_str(&format!(
                "            {field_name}: quent_model::Usage {{\n"
            ));
            lines.push_str(&format!(
                "                resource_id: quent_model::Ref::new(uuid::Uuid::from(data.{field_name}_resource_id)),\n"
            ));
            lines.push_str(&format!(
                "                capacity: {capacity_type} {{\n"
            ));
            for cap in &usage.capacities {
                lines.push_str(&format!(
                    "                    {cap_name}: quent_model::Capacity::new(data.{field_name}_{cap_name}),\n",
                    cap_name = cap.name,
                ));
            }
            lines.push_str("                },\n");
            lines.push_str("            },\n");
        }
    }

    lines.push_str("        };\n");
    lines
}

/// Generate a field-by-field conversion expression from an FFI struct field
/// to the corresponding model type.
fn emit_field_conversion(attr: &AttributeDef) -> String {
    let name = &attr.name;
    match &attr.value_type {
        ValueType::Uuid => format!("            {name}: uuid::Uuid::from(data.{name}),"),
        ValueType::Ref(_) => {
            format!("            {name}: quent_model::Ref::new(uuid::Uuid::from(data.{name})),")
        }
        _ => format!("            {name}: data.{name},"),
    }
}

/// Generate the lib.rs that includes all modules.
fn emit_lib_rs(model: &ModelBuilder, _options: &CxxOptions) -> GeneratedFile {
    let mut mods = String::new();
    mods.push_str("pub mod uuid;\n");
    mods.push_str("pub mod context;\n");
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
