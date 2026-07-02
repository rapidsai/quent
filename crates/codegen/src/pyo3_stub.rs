// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Python type stub generator for PyO3 bridges.

use std::collections::BTreeMap;

use quent_model::{AttributeDef, FsmDef, ModelBuilder, StateDef, UsageDef, ValueType};

use crate::common::{is_auto_declaration_event, resource_operating_attrs, to_pascal_case};
use crate::pyo3_bridge::py_export_name;
use crate::{GeneratedFile, PyO3Options};

fn struct_stub_name(type_path: &str) -> String {
    let last = type_path
        .rsplit("::")
        .next()
        .unwrap_or(type_path)
        .replace(' ', "");
    py_export_name(&format!("{}Dict", to_pascal_case(&last)))
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
        ValueType::List(inner) => format!("list[{}]", py_type(inner, false)),
        ValueType::Struct(type_path, _) => struct_stub_name(type_path),
        ValueType::CustomAttributes => "Mapping[str, bool | int | float | str | None]".to_string(),
    };

    if optional {
        format!("{base} | None")
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
    py_export_name(&format!("{}Handle", to_pascal_case(component_name)))
}

fn observer_class_name(component_name: &str) -> String {
    py_export_name(&format!("{}Observer", to_pascal_case(component_name)))
}

fn usage_type(model: &ModelBuilder, usage: &UsageDef) -> String {
    let handle = handle_class_name(&usage.resource_name);
    let capacity_attrs = resource_operating_attrs(model, usage);
    if capacity_attrs.is_empty() {
        format!("{handle} | None")
    } else {
        let capacity_types = capacity_attrs
            .iter()
            .map(|attr| py_type(&attr.value_type, false))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{handle} | tuple[{handle}, {capacity_types}] | None")
    }
}

fn state_params(model: &ModelBuilder, state: &StateDef) -> Vec<String> {
    let mut params = state
        .attributes
        .iter()
        .map(|attr| {
            format!(
                "{}: {}",
                py_export_name(&attr.name),
                py_type(&attr.value_type, attr.optional)
            )
        })
        .collect::<Vec<_>>();
    params.extend(state.usages.iter().map(|usage| {
        format!(
            "{}: {}",
            py_export_name(&usage.field_name),
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
        out.push_str(&format!("\nclass {name}(TypedDict):\n"));
        if attrs.is_empty() {
            out.push_str("    pass\n");
        } else {
            for attr in attrs {
                out.push_str(&format!(
                    "    {}: {}\n",
                    py_export_name(&attr.name),
                    py_type(&attr.value_type, attr.optional)
                ));
            }
        }
    }
}

fn emit_context(model: &ModelBuilder, out: &mut String) {
    out.push_str("\nclass Context:\n");
    out.push_str(
        "    def __init__(self, exporter: str | None = ..., output_dir: str | None = ...) -> None: ...\n",
    );
    out.push_str("    @property\n");
    out.push_str("    def id(self) -> Uuid: ...\n");
    out.push_str("    def close(self) -> None: ...\n");
    out.push_str("    def __enter__(self) -> Context: ...\n");
    out.push_str("    def __exit__(self, exc_type: object, exc_value: object, traceback: object) -> None: ...\n");
    for entity in &model.entities {
        out.push_str(&format!(
            "    def {}(self) -> {}: ...\n",
            py_export_name(&format!("{}_observer", entity.name)),
            observer_class_name(&entity.name)
        ));
    }
    for fsm in &model.fsms {
        out.push_str(&format!(
            "    def {}(self) -> {}: ...\n",
            py_export_name(&format!("{}_observer", fsm.name)),
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
            py_export_name(&entity.name)
        } else {
            py_export_name(&event.name)
        };
        let return_type = if is_declaration { "Uuid" } else { "None" };
        let params = event
            .attributes
            .iter()
            .map(|attr| {
                format!(
                    "{}: {}",
                    py_export_name(&attr.name),
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
            py_export_name(&entity.name)
        } else {
            py_export_name(&event.name)
        };
        let return_type = if is_declaration { "Uuid" } else { "None" };
        let params = event
            .attributes
            .iter()
            .map(|attr| {
                format!(
                    "{}: {}",
                    py_export_name(&attr.name),
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
        py_export_name(&entry_state.name),
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
            py_export_name(&state.name),
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
    out.push_str("from collections.abc import Mapping\n");
    out.push_str("from typing import TypedDict\n\n");
    out.push_str("def now_v7() -> Uuid: ...\n");
    out.push_str("def nil_uuid() -> Uuid: ...\n");
    out.push_str("\nclass Uuid:\n");
    out.push_str("    def __repr__(self) -> str: ...\n");
    out.push_str("    def __str__(self) -> str: ...\n");
    out.push_str("    def __eq__(self, other: object) -> bool: ...\n");
    out.push_str("    def __hash__(self) -> int: ...\n");

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

    let module_path = options.module_name.replace('.', "/");
    let stub_name = if options.module_name.contains('.') {
        format!("{module_path}.pyi")
    } else {
        format!("{module_path}/__init__.pyi")
    };
    let package_name = options
        .module_name
        .split('.')
        .next()
        .expect("module_name should not be empty");

    vec![
        GeneratedFile {
            name: stub_name,
            content: out,
        },
        GeneratedFile {
            name: format!("{package_name}/py.typed"),
            content: String::new(),
        },
    ]
}
