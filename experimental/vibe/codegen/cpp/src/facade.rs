// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use convert_case::Case;
use quent_ref_target::RefTarget;
use quent_schema::{Annotations, Cardinality, DataType, Entity, Path, Schema};

use crate::common::{cxx_safe, path_pascal, path_snake, to_case};
use crate::{GenerateError, GeneratedFile, Options};

pub(crate) fn emit(schema: &Schema, options: &Options) -> Result<GeneratedFile, GenerateError> {
    let mut output = String::from("#pragma once\n");
    for file in ["uuid", "dynamic_attributes", "context"] {
        output.push_str(&format!(
            "#include \"{}/{}/{}.rs.h\"\n",
            options.crate_name, options.bridge_path, file
        ));
    }
    for entity in schema.entities() {
        output.push_str(&format!(
            "#include \"{}/{}/{}.rs.h\"\n",
            options.crate_name,
            options.bridge_path,
            path_snake(entity.path())
        ));
    }
    output.push_str(
        "#include <cstdint>\n#include <memory>\n#include <optional>\n#include <string>\n#include <utility>\n#include <vector>\n\n",
    );

    emit_common(options, &mut output);
    emit_handle_forwards(schema, options, &mut output);
    emit_value_types(schema, options, &mut output)?;
    emit_event_payloads(schema, options, &mut output);
    emit_conversion_declarations(schema, options, &mut output);
    emit_context_declaration(schema, options, &mut output);
    emit_conversions(schema, options, &mut output);
    emit_handles(schema, options, &mut output);
    emit_context_methods(schema, options, &mut output);

    Ok(GeneratedFile {
        name: "quent.hpp".to_owned(),
        content: output,
    })
}

fn emit_common(options: &Options, output: &mut String) {
    let namespace = &options.namespace;
    output.push_str(&format!(
        r#"namespace {namespace} {{
using Uuid = detail::uuid::UUID;

inline Uuid now_v7() {{ return detail::uuid::now_v7(); }}
inline Uuid nil_uuid() {{ return detail::uuid::new_nil(); }}
inline std::string to_string(const Uuid& id) {{
  return static_cast<std::string>(detail::uuid::to_string(id));
}}

template <typename Entity>
class EntityId final {{
 public:
  explicit EntityId(Uuid value) : value_(value) {{}}
  Uuid raw() const {{ return value_; }}
  friend bool operator==(const EntityId&, const EntityId&) = default;

 private:
  Uuid value_;
}};

namespace facade_detail {{ struct DynamicAttributesAccess; }}

class DynamicAttributes final {{
 public:
  DynamicAttributes() = default;
  DynamicAttributes(DynamicAttributes&&) = default;
  DynamicAttributes& operator=(DynamicAttributes&&) = default;

  void add_null(std::string key) {{
    value_.values.push_back(detail::DynamicAttribute{{
        detail::DynamicAttributeKind::Null, ::rust::String(std::move(key)),
        ::rust::String(), 0, 0, 0.0, false}});
  }}
  void add_string(std::string key, std::string value) {{
    value_.values.push_back(detail::DynamicAttribute{{
        detail::DynamicAttributeKind::String, ::rust::String(std::move(key)),
        ::rust::String(std::move(value)), 0, 0, 0.0, false}});
  }}
  void add_i64(std::string key, std::int64_t value) {{
    value_.values.push_back(detail::DynamicAttribute{{
        detail::DynamicAttributeKind::I64, ::rust::String(std::move(key)),
        ::rust::String(), value, 0, 0.0, false}});
  }}
  void add_u64(std::string key, std::uint64_t value) {{
    value_.values.push_back(detail::DynamicAttribute{{
        detail::DynamicAttributeKind::U64, ::rust::String(std::move(key)),
        ::rust::String(), 0, value, 0.0, false}});
  }}
  void add_f64(std::string key, double value) {{
    value_.values.push_back(detail::DynamicAttribute{{
        detail::DynamicAttributeKind::F64, ::rust::String(std::move(key)),
        ::rust::String(), 0, 0, value, false}});
  }}
  void add_bool(std::string key, bool value) {{
    value_.values.push_back(detail::DynamicAttribute{{
        detail::DynamicAttributeKind::Bool, ::rust::String(std::move(key)),
        ::rust::String(), 0, 0, 0.0, value}});
  }}

 private:
  detail::DynamicAttributes value_;
  friend struct facade_detail::DynamicAttributesAccess;
}};

namespace facade_detail {{
struct DynamicAttributesAccess final {{
  static detail::DynamicAttributes take(DynamicAttributes&& value) {{
    return std::move(value.value_);
  }}
}};
}}  // namespace facade_detail
"#
    ));
    output.push_str(&format!("}}  // namespace {namespace}\n\n"));
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ValueType {
    Record(Path),
    Reference(String),
}

fn emit_value_types(
    schema: &Schema,
    options: &Options,
    output: &mut String,
) -> Result<(), GenerateError> {
    let references = reference_types(schema);
    let mut emitted = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    for record in schema.records() {
        emit_value_type(
            &ValueType::Record(record.path().clone()),
            schema,
            &references,
            options,
            output,
            &mut visiting,
            &mut emitted,
        )?;
    }
    for name in references.keys() {
        emit_value_type(
            &ValueType::Reference(name.clone()),
            schema,
            &references,
            options,
            output,
            &mut visiting,
            &mut emitted,
        )?;
    }
    Ok(())
}

fn emit_value_type(
    value_type: &ValueType,
    schema: &Schema,
    references: &BTreeMap<String, (DataType, Annotations)>,
    options: &Options,
    output: &mut String,
    visiting: &mut BTreeSet<ValueType>,
    emitted: &mut BTreeSet<ValueType>,
) -> Result<(), GenerateError> {
    if emitted.contains(value_type) {
        return Ok(());
    }
    if !visiting.insert(value_type.clone()) {
        return Err(GenerateError::UnsupportedType {
            location: format!("{value_type:?}"),
            reason: "recursive public value types cannot be represented by value in C++".to_owned(),
        });
    }

    match value_type {
        ValueType::Record(path) => {
            let record = schema.record(path).expect("validated record path");
            for field in record.fields() {
                emit_type_dependencies(
                    field.ty(),
                    schema,
                    references,
                    options,
                    output,
                    visiting,
                    emitted,
                )?;
            }
            let namespace = &options.namespace;
            let name = path_pascal(path);
            output.push_str(&format!(
                "namespace {namespace}::records {{\nstruct {name} {{\n"
            ));
            for field in record.fields() {
                output.push_str(&format!(
                    "  {} {};\n",
                    public_type(field.ty(), options),
                    cxx_safe(&to_case(field.name(), Case::Snake))
                ));
            }
            output.push_str(&format!("}};\n}}  // namespace {namespace}::records\n\n"));
        }
        ValueType::Reference(name) => {
            let (data, annotations) = references.get(name).expect("collected reference type");
            emit_type_dependencies(data, schema, references, options, output, visiting, emitted)?;
            let namespace = &options.namespace;
            output.push_str(&format!(
                "namespace {namespace}::refs {{\nstruct {name} {{\n  {} target;\n  {} data;\n}};\n}}  // namespace {namespace}::refs\n\n",
                reference_target_type(annotations, options),
                public_type(data, options)
            ));
        }
    }
    visiting.remove(value_type);
    emitted.insert(value_type.clone());
    Ok(())
}

fn emit_type_dependencies(
    ty: &DataType,
    schema: &Schema,
    references: &BTreeMap<String, (DataType, Annotations)>,
    options: &Options,
    output: &mut String,
    visiting: &mut BTreeSet<ValueType>,
    emitted: &mut BTreeSet<ValueType>,
) -> Result<(), GenerateError> {
    match ty {
        DataType::Option(inner) | DataType::List(inner) => emit_type_dependencies(
            inner, schema, references, options, output, visiting, emitted,
        ),
        DataType::Record(path) => emit_value_type(
            &ValueType::Record(path.clone()),
            schema,
            references,
            options,
            output,
            visiting,
            emitted,
        ),
        DataType::EntityRef {
            data: Some(data),
            annotations,
        } => emit_value_type(
            &ValueType::Reference(reference_name(data, annotations)),
            schema,
            references,
            options,
            output,
            visiting,
            emitted,
        ),
        _ => Ok(()),
    }
}

fn emit_event_payloads(schema: &Schema, options: &Options, output: &mut String) {
    for entity in schema.entities() {
        let namespace = public_entity_namespace(entity, options);
        output.push_str(&format!("namespace {namespace} {{\n"));
        for event in entity.events() {
            if event.fields().next().is_none() {
                continue;
            }
            output.push_str(&format!(
                "struct {} {{\n",
                to_case(event.name(), Case::Pascal)
            ));
            for field in event.fields() {
                output.push_str(&format!(
                    "  {} {};\n",
                    public_type(field.ty(), options),
                    cxx_safe(&to_case(field.name(), Case::Snake))
                ));
            }
            output.push_str("};\n\n");
        }
        output.push_str(&format!("}}  // namespace {namespace}\n\n"));
    }
}

fn emit_conversion_declarations(schema: &Schema, options: &Options, output: &mut String) {
    let namespace = &options.namespace;
    output.push_str(&format!("namespace {namespace}::facade_detail {{\n"));
    for entity in schema.entities() {
        let prefix = path_snake(entity.path());
        let raw_namespace = raw_entity_namespace(entity, options);
        for path in used_records(schema, entity) {
            let record = schema.record(&path).expect("collected schema record");
            let name = path_pascal(record.path());
            let raw_name = format!("BridgeRecord{name}");
            output.push_str(&format!(
                "inline ::{raw_namespace}::{raw_name} {prefix}_to_raw_{name}(::{namespace}::records::{name} value);\n"
            ));
        }
        for event in entity.events() {
            if event.fields().next().is_none() {
                continue;
            }
            let name = to_case(event.name(), Case::Pascal);
            let public_namespace = public_entity_namespace(entity, options);
            output.push_str(&format!(
                "inline ::{raw_namespace}::{name} {prefix}_to_raw_{name}(::{public_namespace}::{name} value);\n"
            ));
        }
    }
    output.push_str(&format!("}}  // namespace {namespace}::facade_detail\n\n"));
}

fn emit_handle_forwards(schema: &Schema, options: &Options, output: &mut String) {
    for entity in schema.entities() {
        let namespace = public_entity_namespace(entity, options);
        let name = path_pascal(entity.path());
        output.push_str(&format!(
            "namespace {namespace} {{ struct {name}Tag; using {name}Id = ::{}::EntityId<{name}Tag>; class {name}Observer; class {name}Handle; }}\n",
            options.namespace,
        ));
    }
    output.push('\n');
}

fn emit_context_declaration(schema: &Schema, options: &Options, output: &mut String) {
    let namespace = &options.namespace;
    output.push_str(&format!(
        r#"namespace {namespace} {{
class Context final {{
 public:
  Context(Context&&) = default;
  Context& operator=(Context&&) = default;

  static Context none();
"#
    ));
    if options.exporters.ndjson {
        output.push_str("  static Context ndjson(std::string output_dir);\n");
    }
    if options.exporters.msgpack {
        output.push_str("  static Context msgpack(std::string output_dir);\n");
    }
    if options.exporters.postcard {
        output.push_str("  static Context postcard(std::string output_dir);\n");
    }
    if options.exporters.collector {
        output.push_str("  static Context collector(std::string address);\n");
    }
    output.push_str("\n  Uuid id() const { return inner_->id(); }\n");
    for entity in schema.entities() {
        let observer = format!(
            "::{}::{}Observer",
            public_entity_namespace(entity, options),
            path_pascal(entity.path())
        );
        let method = cxx_safe(&format!("{}_observer", path_snake(entity.path())));
        output.push_str(&format!(
            "  std::shared_ptr<{observer}> {method}() const;\n"
        ));
    }
    output.push_str(&format!(
        r#"
 private:
  explicit Context(::rust::Box<detail::Context> inner) : inner_(std::move(inner)) {{}}
  ::rust::Box<detail::Context> inner_;
}};
}}  // namespace {namespace}

"#
    ));
}

fn emit_conversions(schema: &Schema, options: &Options, output: &mut String) {
    let namespace = &options.namespace;
    output.push_str(&format!("namespace {namespace}::facade_detail {{\n"));
    for entity in schema.entities() {
        let prefix = path_snake(entity.path());
        let raw_namespace = raw_entity_namespace(entity, options);
        for path in used_records(schema, entity) {
            let record = schema.record(&path).expect("collected schema record");
            let name = path_pascal(record.path());
            let raw_name = format!("BridgeRecord{name}");
            output.push_str(&format!(
                "inline ::{raw_namespace}::{raw_name} {prefix}_to_raw_{name}(::{namespace}::records::{name} value) {{\n"
            ));
            if record.fields().next().is_none() {
                output.push_str(&format!(
                    "  static_cast<void>(value);\n  return ::{raw_namespace}::{raw_name}{{}};\n"
                ));
            } else {
                output.push_str(&format!("  return ::{raw_namespace}::{raw_name}{{\n"));
                for field in record.fields() {
                    let field_name = cxx_safe(&to_case(field.name(), Case::Snake));
                    output.push_str(&format!(
                        "    {},\n",
                        conversion(
                            field.ty(),
                            &format!("std::move(value.{field_name})"),
                            entity,
                            options,
                        )
                    ));
                }
                output.push_str("  };\n");
            }
            output.push_str("}\n\n");
        }
        for event in entity.events() {
            if event.fields().next().is_none() {
                continue;
            }
            let name = to_case(event.name(), Case::Pascal);
            let public_namespace = public_entity_namespace(entity, options);
            output.push_str(&format!(
                "inline ::{raw_namespace}::{name} {prefix}_to_raw_{name}(::{public_namespace}::{name} value) {{\n  return ::{raw_namespace}::{name}{{\n"
            ));
            for field in event.fields() {
                let field_name = cxx_safe(&to_case(field.name(), Case::Snake));
                output.push_str(&format!(
                    "    {},\n",
                    conversion(
                        field.ty(),
                        &format!("std::move(value.{field_name})"),
                        entity,
                        options,
                    )
                ));
            }
            output.push_str("  };\n}\n\n");
        }
    }
    output.push_str(&format!("}}  // namespace {namespace}::facade_detail\n\n"));
}

fn emit_handles(schema: &Schema, options: &Options, output: &mut String) {
    let base_namespace = &options.namespace;
    for entity in schema.entities() {
        let namespace = public_entity_namespace(entity, options);
        let raw_namespace = raw_entity_namespace(entity, options);
        let name = path_pascal(entity.path());
        let raw_observer = format!("{name}Observer");
        let raw_handle = format!("{name}Handle");
        let prefix = path_snake(entity.path());
        output.push_str(&format!(
            "namespace {namespace} {{\nclass {name}Observer final {{\n public:\n  {name}Handle create() const;\n  {name}Handle create({name}Id id) const;\n\n private:\n  explicit {name}Observer(::rust::Box<::{raw_namespace}::{raw_observer}> inner) : inner_(std::move(inner)) {{}}\n  ::rust::Box<::{raw_namespace}::{raw_observer}> inner_;\n  friend class ::{base_namespace}::Context;\n}};\n\nclass {name}Handle final {{\n public:\n  {name}Handle({name}Handle&&) = default;\n  {name}Handle& operator=({name}Handle&&) = default;\n  {name}Id id() const {{ return {name}Id(inner_->uuid()); }}\n"
        ));
        for event in entity.events() {
            let method = cxx_safe(&to_case(event.name(), Case::Snake));
            let constness = if event.cardinality() == Cardinality::Multi {
                " const"
            } else {
                ""
            };
            if event.fields().next().is_none() {
                output.push_str(&format!(
                    "  void {method}(){constness} {{ inner_->{method}(); }}\n"
                ));
            } else {
                let payload = to_case(event.name(), Case::Pascal);
                output.push_str(&format!(
                    "  void {method}({payload} data){constness} {{ inner_->{method}(::{base_namespace}::facade_detail::{prefix}_to_raw_{payload}(std::move(data))); }}\n"
                ));
            }
            if event.cardinality() == Cardinality::Once {
                let emitted = cxx_safe(&format!("{}_emitted", to_case(event.name(), Case::Snake)));
                output.push_str(&format!(
                    "  bool {emitted}() const {{ return inner_->{emitted}(); }}\n"
                ));
            }
        }
        output.push_str(&format!(
            "\n private:\n  explicit {name}Handle(::rust::Box<::{raw_namespace}::{raw_handle}> inner) : inner_(std::move(inner)) {{}}\n  ::rust::Box<::{raw_namespace}::{raw_handle}> inner_;\n  friend class {name}Observer;\n}};\n\ninline {name}Handle {name}Observer::create() const {{\n  return {name}Handle(inner_->create());\n}}\ninline {name}Handle {name}Observer::create({name}Id id) const {{\n  return {name}Handle(inner_->create_with_id(id.raw()));\n}}\n}}  // namespace {namespace}\n\n"
        ));
    }
}

fn emit_context_methods(schema: &Schema, options: &Options, output: &mut String) {
    let namespace = &options.namespace;
    output.push_str(&format!(
        r#"namespace {namespace} {{
inline Context Context::none() {{
  return Context(detail::create_context(detail::ExporterOptions::none()));
}}
"#
    ));
    if options.exporters.ndjson {
        output.push_str(
            "inline Context Context::ndjson(std::string output_dir) {\n  return Context(detail::create_context(\n      detail::ExporterOptions::ndjson(::rust::String(std::move(output_dir)))));\n}\n",
        );
    }
    if options.exporters.msgpack {
        output.push_str(
            "inline Context Context::msgpack(std::string output_dir) {\n  return Context(detail::create_context(\n      detail::ExporterOptions::msgpack(::rust::String(std::move(output_dir)))));\n}\n",
        );
    }
    if options.exporters.postcard {
        output.push_str(
            "inline Context Context::postcard(std::string output_dir) {\n  return Context(detail::create_context(\n      detail::ExporterOptions::postcard(::rust::String(std::move(output_dir)))));\n}\n",
        );
    }
    if options.exporters.collector {
        output.push_str(
            "inline Context Context::collector(std::string address) {\n  return Context(detail::create_context(\n      detail::ExporterOptions::collector(::rust::String(std::move(address)))));\n}\n",
        );
    }
    for entity in schema.entities() {
        let public_namespace = public_entity_namespace(entity, options);
        let raw_namespace = raw_entity_namespace(entity, options);
        let name = path_pascal(entity.path());
        let method = cxx_safe(&format!("{}_observer", path_snake(entity.path())));
        output.push_str(&format!(
            "inline std::shared_ptr<::{public_namespace}::{name}Observer> Context::{method}() const {{\n  return std::shared_ptr<::{public_namespace}::{name}Observer>(\n      new ::{public_namespace}::{name}Observer(\n          ::{raw_namespace}::create_observer(*inner_)));\n}}\n"
        ));
    }
    output.push_str(&format!("}}  // namespace {namespace}\n"));
}

fn public_type(ty: &DataType, options: &Options) -> String {
    let namespace = &options.namespace;
    match ty {
        DataType::Bool => "bool".to_owned(),
        DataType::Uuid => format!("::{namespace}::Uuid"),
        DataType::String => "std::string".to_owned(),
        DataType::U8 => "std::uint8_t".to_owned(),
        DataType::U16 => "std::uint16_t".to_owned(),
        DataType::U32 => "std::uint32_t".to_owned(),
        DataType::U64 => "std::uint64_t".to_owned(),
        DataType::I8 => "std::int8_t".to_owned(),
        DataType::I16 => "std::int16_t".to_owned(),
        DataType::I32 => "std::int32_t".to_owned(),
        DataType::I64 => "std::int64_t".to_owned(),
        DataType::F32 => "float".to_owned(),
        DataType::F64 => "double".to_owned(),
        DataType::Option(inner) => format!("std::optional<{}>", public_type(inner, options)),
        DataType::List(inner) => format!("std::vector<{}>", public_type(inner, options)),
        DataType::Record(path) => format!("::{namespace}::records::{}", path_pascal(path)),
        DataType::DynamicRecord => format!("::{namespace}::DynamicAttributes"),
        DataType::EntityRef { data, annotations } => match data {
            Some(data) => format!("::{namespace}::refs::{}", reference_name(data, annotations)),
            None => reference_target_type(annotations, options),
        },
    }
}

fn conversion(ty: &DataType, expression: &str, entity: &Entity, options: &Options) -> String {
    let namespace = &options.namespace;
    let prefix = path_snake(entity.path());
    match ty {
        DataType::Bool
        | DataType::U8
        | DataType::U16
        | DataType::U32
        | DataType::U64
        | DataType::I8
        | DataType::I16
        | DataType::I32
        | DataType::I64
        | DataType::F32
        | DataType::F64
        | DataType::Uuid => expression.to_owned(),
        DataType::String => format!("::rust::String({expression})"),
        DataType::DynamicRecord => format!(
            "::{namespace}::facade_detail::DynamicAttributesAccess::take(std::move({expression}))"
        ),
        DataType::Record(path) => format!(
            "::{namespace}::facade_detail::{prefix}_to_raw_{}({expression})",
            path_pascal(path)
        ),
        DataType::Option(inner) => {
            let raw = raw_type(ty, entity, options);
            let converted = conversion(inner, "std::move(*input)", entity, options);
            format!(
                "[] ({} input) {{ {raw} output{{}}; if (input) {{ output.has_value = true; output.value = {converted}; }} return output; }}({expression})",
                public_type(ty, options)
            )
        }
        DataType::List(inner) => {
            let raw_item = raw_list_item_type(inner, entity, options);
            let converted = conversion(inner, "std::move(item)", entity, options);
            let converted = if matches!(inner.as_ref(), DataType::List(_)) {
                format!("{raw_item}{{{converted}}}")
            } else {
                converted
            };
            format!(
                "[] ({} input) {{ ::rust::Vec<{raw_item}> output; output.reserve(input.size()); for (auto& item : input) {{ output.push_back({converted}); }} return output; }}({expression})",
                public_type(ty, options)
            )
        }
        DataType::EntityRef { data, annotations } => match data {
            None => {
                if RefTarget::from_annotations(annotations).is_some() {
                    format!("({expression}).raw()")
                } else {
                    expression.to_owned()
                }
            }
            Some(data) => {
                let raw = raw_type(ty, entity, options);
                let converted = conversion(data, "std::move(value.data)", entity, options);
                let target = if RefTarget::from_annotations(annotations).is_some() {
                    "value.target.raw()"
                } else {
                    "value.target"
                };
                format!(
                    "[] ({} value) {{ return {raw}{{{target}, {converted}}}; }}({expression})",
                    public_type(ty, options)
                )
            }
        },
    }
}

fn raw_type(ty: &DataType, entity: &Entity, options: &Options) -> String {
    let namespace = &options.namespace;
    let raw_namespace = raw_entity_namespace(entity, options);
    match ty {
        DataType::Bool => "bool".to_owned(),
        DataType::Uuid | DataType::EntityRef { data: None, .. } => {
            format!("::{namespace}::detail::uuid::UUID")
        }
        DataType::String => "::rust::String".to_owned(),
        DataType::U8 => "std::uint8_t".to_owned(),
        DataType::U16 => "std::uint16_t".to_owned(),
        DataType::U32 => "std::uint32_t".to_owned(),
        DataType::U64 => "std::uint64_t".to_owned(),
        DataType::I8 => "std::int8_t".to_owned(),
        DataType::I16 => "std::int16_t".to_owned(),
        DataType::I32 => "std::int32_t".to_owned(),
        DataType::I64 => "std::int64_t".to_owned(),
        DataType::F32 => "float".to_owned(),
        DataType::F64 => "double".to_owned(),
        DataType::Option(inner) => {
            format!("::{raw_namespace}::BridgeOptional{}", type_key(inner))
        }
        DataType::List(inner) => {
            format!(
                "::rust::Vec<{}>",
                raw_list_item_type(inner, entity, options)
            )
        }
        DataType::Record(path) => {
            format!("::{raw_namespace}::BridgeRecord{}", path_pascal(path))
        }
        DataType::DynamicRecord => format!("::{namespace}::detail::DynamicAttributes"),
        DataType::EntityRef {
            data: Some(data),
            annotations,
        } => {
            let target = RefTarget::from_annotations(annotations)
                .map(|target| path_pascal(target.as_ref()))
                .unwrap_or_else(|| "AnyEntity".to_owned());
            format!(
                "::{raw_namespace}::BridgeReference{target}{}",
                type_key(data)
            )
        }
    }
}

fn raw_list_item_type(inner: &DataType, entity: &Entity, options: &Options) -> String {
    if matches!(inner, DataType::List(_)) {
        format!(
            "::{}::BridgeList{}",
            raw_entity_namespace(entity, options),
            type_key(inner)
        )
    } else {
        raw_type(inner, entity, options)
    }
}

fn reference_types(schema: &Schema) -> BTreeMap<String, (DataType, Annotations)> {
    let mut output = BTreeMap::new();
    for record in schema.records() {
        for field in record.fields() {
            collect_reference_types(field.ty(), &mut output);
        }
    }
    for entity in schema.entities() {
        for event in entity.events() {
            for field in event.fields() {
                collect_reference_types(field.ty(), &mut output);
            }
        }
    }
    output
}

fn used_records(schema: &Schema, entity: &Entity) -> Vec<Path> {
    let mut output = Vec::new();
    for event in entity.events() {
        for field in event.fields() {
            collect_used_records(schema, field.ty(), &mut output);
        }
    }
    output
}

fn collect_used_records(schema: &Schema, ty: &DataType, output: &mut Vec<Path>) {
    match ty {
        DataType::Option(inner) | DataType::List(inner) => {
            collect_used_records(schema, inner, output);
        }
        DataType::Record(path) => {
            if output.contains(path) {
                return;
            }
            output.push(path.clone());
            let record = schema.record(path).expect("validated record reference");
            for field in record.fields() {
                collect_used_records(schema, field.ty(), output);
            }
        }
        DataType::EntityRef {
            data: Some(data), ..
        } => collect_used_records(schema, data, output),
        _ => {}
    }
}

fn collect_reference_types(ty: &DataType, output: &mut BTreeMap<String, (DataType, Annotations)>) {
    match ty {
        DataType::Option(inner) | DataType::List(inner) => collect_reference_types(inner, output),
        DataType::EntityRef {
            data: Some(data),
            annotations,
        } => {
            output
                .entry(reference_name(data, annotations))
                .or_insert_with(|| (data.as_ref().clone(), annotations.clone()));
            collect_reference_types(data, output);
        }
        _ => {}
    }
}

pub(crate) fn reference_name(data: &DataType, annotations: &Annotations) -> String {
    let target = RefTarget::from_annotations(annotations)
        .map(|target| path_pascal(target.as_ref()))
        .unwrap_or_else(|| "AnyEntity".to_owned());
    let data = type_key(data);
    if data
        .strip_prefix(&target)
        .is_some_and(|suffix| !suffix.is_empty())
    {
        format!("{data}Ref")
    } else {
        format!("{target}{data}Ref")
    }
}

fn reference_target_type(annotations: &Annotations, options: &Options) -> String {
    let namespace = &options.namespace;
    RefTarget::from_annotations(annotations)
        .map(|target| {
            let path = target.as_ref();
            format!(
                "::{}::{}Id",
                entity_namespace(namespace, path),
                path_pascal(path)
            )
        })
        .unwrap_or_else(|| format!("::{namespace}::Uuid"))
}

fn type_key(ty: &DataType) -> String {
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
        DataType::Option(inner) => format!("Optional{}", type_key(inner)),
        DataType::List(inner) => format!("{}List", type_key(inner)),
        DataType::Record(path) => path_pascal(path),
        DataType::DynamicRecord => "DynamicAttributes".to_owned(),
        DataType::EntityRef { data, annotations } => match data {
            Some(data) => reference_name(data, annotations),
            None => RefTarget::from_annotations(annotations)
                .map(|target| format!("{}Ref", path_pascal(target.as_ref())))
                .unwrap_or_else(|| "AnyEntityRef".to_owned()),
        },
    }
}

fn public_entity_namespace(entity: &Entity, options: &Options) -> String {
    entity_namespace(&options.namespace, entity.path())
}

fn raw_entity_namespace(entity: &Entity, options: &Options) -> String {
    entity_namespace(&format!("{}::detail", options.namespace), entity.path())
}

fn entity_namespace(base: &str, path: &Path) -> String {
    std::iter::once(base.to_owned())
        .chain(
            path.namespace()
                .iter()
                .chain(std::iter::once(path.name()))
                .map(|part| cxx_safe(&to_case(part, Case::Snake))),
        )
        .collect::<Vec<_>>()
        .join("::")
}
