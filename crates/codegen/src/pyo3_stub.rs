// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Python type stub generator for PyO3 bridges.

use std::collections::BTreeMap;

use quent_model::{AttributeDef, FsmDef, ModelBuilder, StateDef, UsageDef, ValueType};

use crate::common::{is_auto_declaration_event, resource_operating_attrs, to_pascal_case};
use crate::{GeneratedFile, PyO3Options};

fn sanitize_py_ident(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if (i == 0 && (ch == '_' || ch.is_ascii_alphabetic()))
            || (i > 0 && (ch == '_' || ch.is_ascii_alphanumeric()))
        {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || out.as_bytes()[0].is_ascii_digit() {
        out.insert(0, '_');
    }
    out
}

fn struct_stub_name(type_path: &str) -> String {
    let last = type_path
        .rsplit("::")
        .next()
        .unwrap_or(type_path)
        .replace(' ', "");
    format!("{}Dict", sanitize_py_ident(&to_pascal_case(&last)))
}

fn py_type(ty: &ValueType, optional: bool) -> String {
    let base = match ty {
        ValueType::Bool => "bool".to_string(),
        ValueType::Uuid | ValueType::Ref(_) => "Uuid".to_string(),
        ValueType::String => "str".to_string(),
        ValueType::U8
        | ValueType::U16
        | ValueType::U32
        | ValueType::U64
        | ValueType::I8
        | ValueType::I16
        | ValueType::I32
        | ValueType::I64 => "int".to_string(),
        ValueType::F32 | ValueType::F64 => "float".to_string(),
        ValueType::List(inner) => format!("typing.List[{}]", py_type(inner, false)),
        ValueType::Struct(type_path, _) => struct_stub_name(type_path),
        ValueType::CustomAttributes => {
            "typing.Union[CustomAttributes, typing.Mapping[str, object]]".to_string()
        }
    };

    if optional {
        format!("typing.Optional[{base}]")
    } else {
        base
    }
}

fn collect_struct_defs(attrs: &[AttributeDef], structs: &mut BTreeMap<String, Vec<AttributeDef>>) {
    for attr in attrs {
        match &attr.value_type {
            ValueType::Struct(type_path, inner) => {
                structs
                    .entry(struct_stub_name(type_path))
                    .or_insert_with(|| inner.clone());
                collect_struct_defs(inner, structs);
            }
            ValueType::List(inner) => {
                if let ValueType::Struct(type_path, inner_attrs) = inner.as_ref() {
                    structs
                        .entry(struct_stub_name(type_path))
                        .or_insert_with(|| inner_attrs.clone());
                    collect_struct_defs(inner_attrs, structs);
                }
            }
            _ => {}
        }
    }
}

fn function_params(params: &[String]) -> String {
    if params.is_empty() {
        String::new()
    } else {
        format!(", {}", params.join(", "))
    }
}

fn handle_class_name(component_name: &str) -> String {
    format!("{}Handle", to_pascal_case(component_name))
}

fn observer_class_name(component_name: &str) -> String {
    format!("{}Observer", to_pascal_case(component_name))
}

fn usage_type(model: &ModelBuilder, usage: &UsageDef) -> String {
    let handle = handle_class_name(&usage.resource_name);
    let capacity_attrs = resource_operating_attrs(model, usage);
    if capacity_attrs.is_empty() {
        format!("typing.Optional[{handle}]")
    } else {
        let capacity_types = capacity_attrs
            .iter()
            .map(|attr| py_type(&attr.value_type, false))
            .collect::<Vec<_>>()
            .join(", ");
        format!("typing.Optional[typing.Union[{handle}, typing.Tuple[{handle}, {capacity_types}]]]")
    }
}

fn state_params(model: &ModelBuilder, state: &StateDef) -> Vec<String> {
    let mut params = state
        .attributes
        .iter()
        .map(|attr| {
            format!(
                "{}: {}",
                sanitize_py_ident(&attr.name),
                py_type(&attr.value_type, attr.optional)
            )
        })
        .collect::<Vec<_>>();
    params.extend(state.usages.iter().map(|usage| {
        format!(
            "{}: {}",
            sanitize_py_ident(&usage.field_name),
            usage_type(model, usage)
        )
    }));
    params
}

fn emit_struct_definitions(model: &ModelBuilder, out: &mut String) {
    let mut structs = BTreeMap::new();
    for entity in &model.entities {
        for event in &entity.events {
            collect_struct_defs(&event.attributes, &mut structs);
        }
    }
    for fsm in &model.fsms {
        for state in &fsm.states {
            collect_struct_defs(&state.attributes, &mut structs);
        }
    }

    for (name, attrs) in structs {
        out.push_str(&format!("\nclass {name}(typing.TypedDict):\n"));
        if attrs.is_empty() {
            out.push_str("    pass\n");
        } else {
            for attr in attrs {
                out.push_str(&format!(
                    "    {}: {}\n",
                    sanitize_py_ident(&attr.name),
                    py_type(&attr.value_type, attr.optional)
                ));
            }
        }
    }
}

fn emit_context(model: &ModelBuilder, out: &mut String) {
    out.push_str("\nclass Context:\n");
    out.push_str(
        "    def __init__(self, id: Uuid, exporter: typing.Optional[str] = ..., output_dir: typing.Optional[str] = ...) -> None: ...\n",
    );
    out.push_str("    @property\n");
    out.push_str("    def id(self) -> Uuid: ...\n");
    out.push_str("    def close(self) -> None: ...\n");
    out.push_str("    def __enter__(self) -> Context: ...\n");
    out.push_str("    def __exit__(self, exc_type: object, exc_value: object, traceback: object) -> None: ...\n");
    for entity in &model.entities {
        out.push_str(&format!(
            "    def {}_observer(self) -> {}: ...\n",
            sanitize_py_ident(&entity.name),
            observer_class_name(&entity.name)
        ));
    }
    for fsm in &model.fsms {
        out.push_str(&format!(
            "    def {}_observer(self) -> {}: ...\n",
            sanitize_py_ident(&fsm.name),
            observer_class_name(&fsm.name)
        ));
    }
}

fn emit_entity_observer(entity: &quent_model::EntityDef, out: &mut String) {
    out.push_str(&format!("\nclass {}:\n", observer_class_name(&entity.name)));
    if entity.events.is_empty() {
        out.push_str("    pass\n");
        return;
    }

    if entity.events.len() > 1 {
        out.push_str(&format!(
            "    def create(self, id: Uuid) -> {}: ...\n",
            handle_class_name(&entity.name)
        ));
        return;
    }

    for event in &entity.events {
        let is_declaration = is_auto_declaration_event(&entity.name, &event.name);
        let method_name = if is_declaration {
            sanitize_py_ident(&entity.name)
        } else {
            sanitize_py_ident(&event.name)
        };
        let return_type = if is_declaration { "Uuid" } else { "None" };
        let params = event
            .attributes
            .iter()
            .map(|attr| {
                format!(
                    "{}: {}",
                    sanitize_py_ident(&attr.name),
                    py_type(&attr.value_type, attr.optional)
                )
            })
            .collect::<Vec<_>>();
        out.push_str(&format!(
            "    def {}(self, id: Uuid{}) -> {}: ...\n",
            method_name,
            function_params(&params),
            return_type
        ));
    }
}

fn emit_entity_handle(entity: &quent_model::EntityDef, out: &mut String) {
    if entity.events.len() <= 1 {
        return;
    }

    out.push_str(&format!("\nclass {}:\n", handle_class_name(&entity.name)));
    out.push_str("    @property\n");
    out.push_str("    def uuid(self) -> Uuid: ...\n");
    for event in &entity.events {
        let is_declaration = is_auto_declaration_event(&entity.name, &event.name);
        let method_name = if is_declaration {
            sanitize_py_ident(&entity.name)
        } else {
            sanitize_py_ident(&event.name)
        };
        let return_type = if is_declaration { "Uuid" } else { "None" };
        let params = event
            .attributes
            .iter()
            .map(|attr| {
                format!(
                    "{}: {}",
                    sanitize_py_ident(&attr.name),
                    py_type(&attr.value_type, attr.optional)
                )
            })
            .collect::<Vec<_>>();
        out.push_str(&format!(
            "    def {}(self{}) -> {}: ...\n",
            method_name,
            function_params(&params),
            return_type
        ));
    }
}

fn emit_fsm_observer(model: &ModelBuilder, fsm: &FsmDef, out: &mut String) {
    let Some(entry_state) = fsm.states.iter().find(|state| state.name == fsm.entry) else {
        return;
    };
    let params = state_params(model, entry_state);
    out.push_str(&format!("\nclass {}:\n", observer_class_name(&fsm.name)));
    out.push_str(&format!(
        "    def {}(self, id: Uuid{}) -> {}: ...\n",
        sanitize_py_ident(&entry_state.name),
        function_params(&params),
        handle_class_name(&fsm.name)
    ));
}

fn emit_fsm_handle(model: &ModelBuilder, fsm: &FsmDef, out: &mut String) {
    out.push_str(&format!("\nclass {}:\n", handle_class_name(&fsm.name)));
    out.push_str("    @property\n");
    out.push_str("    def uuid(self) -> Uuid: ...\n");
    for state in &fsm.states {
        if state.name == fsm.entry {
            continue;
        }
        let params = state_params(model, state);
        out.push_str(&format!(
            "    def {}(self{}) -> None: ...\n",
            sanitize_py_ident(&state.name),
            function_params(&params)
        ));
    }
    out.push_str("    def exit(self) -> None: ...\n");
}

/// Generate `.pyi` files for a PyO3 bridge.
pub fn emit(model: &ModelBuilder, options: &PyO3Options) -> Vec<GeneratedFile> {
    let mut out = String::new();
    out.push_str("# Generated by quent-codegen. Do not edit by hand.\n");
    out.push_str("from __future__ import annotations\n\n");
    out.push_str("import typing\n\n");
    out.push_str("def now_v7() -> Uuid: ...\n");
    out.push_str("def nil_uuid() -> Uuid: ...\n");
    out.push_str("\nclass Uuid:\n");
    out.push_str("    def __repr__(self) -> str: ...\n");
    out.push_str("    def __str__(self) -> str: ...\n");
    out.push_str("    def __eq__(self, other: object) -> bool: ...\n");
    out.push_str("    def __hash__(self) -> int: ...\n");
    out.push_str("\nclass CustomAttributes:\n");
    out.push_str("    def __init__(self) -> None: ...\n");
    out.push_str("    def add_string(self, key: str, value: str) -> None: ...\n");
    out.push_str("    def add_u64(self, key: str, value: int) -> None: ...\n");
    out.push_str("    def add_i64(self, key: str, value: int) -> None: ...\n");
    out.push_str("    def add_f64(self, key: str, value: float) -> None: ...\n");
    out.push_str("    def add_bool(self, key: str, value: bool) -> None: ...\n");

    emit_struct_definitions(model, &mut out);
    emit_context(model, &mut out);
    for entity in &model.entities {
        emit_entity_observer(entity, &mut out);
        emit_entity_handle(entity, &mut out);
    }
    for fsm in &model.fsms {
        emit_fsm_observer(model, fsm, &mut out);
        emit_fsm_handle(model, fsm, &mut out);
    }

    vec![GeneratedFile {
        name: format!("{}.pyi", options.module_name.replace('.', "/")),
        content: out,
    }]
}
