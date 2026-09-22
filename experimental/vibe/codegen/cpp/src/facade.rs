// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use convert_case::Case;
use quent_fsm::{Fsm, SEQUENCE_FIELD_NAME};
use quent_ref_target::RefTarget;
use quent_schema::{Annotations, Cardinality, DataType, Entity, Event, Field, Path, Schema};

use crate::common::{cxx_safe, path_pascal, path_snake, to_case};
use crate::{GenerateError, GeneratedFile, Options, has_nvtx_source, public_entities};

pub(crate) fn emit(schema: &Schema, options: &Options) -> Result<GeneratedFile, GenerateError> {
    let mut output = String::from("#pragma once\n");
    for file in ["uuid", "dynamic_attributes", "context"] {
        output.push_str(&format!(
            "#include \"{}/{}/{}.rs.h\"\n",
            options.crate_name, options.bridge_path, file
        ));
    }
    for entity in public_entities(schema) {
        output.push_str(&format!(
            "#include \"{}/{}/{}.rs.h\"\n",
            options.crate_name,
            options.bridge_path,
            path_snake(entity.path())
        ));
    }
    output.push_str(
        "#include <cstddef>\n#include <cstdint>\n#include <memory>\n#include <optional>\n#include <string>\n#include <utility>\n#include <vector>\n\n",
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

namespace facade_detail {{ struct InitialFsmState final {{}}; }}

template <typename Entity>
class Handle;

template <typename Entity, typename State = facade_detail::InitialFsmState>
class FsmHandle;

namespace facade_detail {{
struct DynamicAttributesAccess;
struct HandleAccess final {{
  template <typename PublicHandle, typename Inner>
  static PublicHandle make(Inner&& inner) {{
    return PublicHandle(std::forward<Inner>(inner));
  }}
}};
}}

class DynamicList;

class DynamicAttributes final {{
 public:
  DynamicAttributes() : value_(detail::dynamic_attributes_new()) {{}}
  DynamicAttributes(DynamicAttributes&& other) noexcept
      : value_(std::move(other.value_)) {{
    other.value_ = detail::dynamic_attributes_new();
  }}
  DynamicAttributes& operator=(DynamicAttributes&& other) noexcept {{
    if (this != &other) {{
      value_ = std::move(other.value_);
      other.value_ = detail::dynamic_attributes_new();
    }}
    return *this;
  }}

  void add(std::string key, std::nullptr_t) {{
    value_->dynamic_attributes_add_null(::rust::String(std::move(key)));
  }}
  void add(std::string key, std::uint8_t value) {{
    value_->dynamic_attributes_add_u8(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, std::uint16_t value) {{
    value_->dynamic_attributes_add_u16(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, std::uint32_t value) {{
    value_->dynamic_attributes_add_u32(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, std::uint64_t value) {{
    value_->dynamic_attributes_add_u64(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, std::int8_t value) {{
    value_->dynamic_attributes_add_i8(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, std::int16_t value) {{
    value_->dynamic_attributes_add_i16(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, std::int32_t value) {{
    value_->dynamic_attributes_add_i32(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, std::int64_t value) {{
    value_->dynamic_attributes_add_i64(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, float value) {{
    value_->dynamic_attributes_add_f32(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, double value) {{
    value_->dynamic_attributes_add_f64(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, std::string value) {{
    value_->dynamic_attributes_add_string(
        ::rust::String(std::move(key)), ::rust::String(std::move(value)));
  }}
  void add(std::string key, const char* value) {{
    add(std::move(key), std::string(value));
  }}
  void add(std::string key, bool value) {{
    value_->dynamic_attributes_add_bool(::rust::String(std::move(key)), value);
  }}
  void add(std::string key, DynamicAttributes value) {{
    value_->dynamic_attributes_add_structure(
        ::rust::String(std::move(key)), std::move(value.value_));
  }}
  void add(std::string key, std::vector<std::uint8_t> values) {{
    value_->dynamic_attributes_add_u8_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<std::uint16_t> values) {{
    value_->dynamic_attributes_add_u16_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<std::uint32_t> values) {{
    value_->dynamic_attributes_add_u32_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<std::uint64_t> values) {{
    value_->dynamic_attributes_add_u64_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<std::int8_t> values) {{
    value_->dynamic_attributes_add_i8_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<std::int16_t> values) {{
    value_->dynamic_attributes_add_i16_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<std::int32_t> values) {{
    value_->dynamic_attributes_add_i32_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<std::int64_t> values) {{
    value_->dynamic_attributes_add_i64_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<float> values) {{
    value_->dynamic_attributes_add_f32_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<double> values) {{
    value_->dynamic_attributes_add_f64_list(
        ::rust::String(std::move(key)), into_rust_vec(std::move(values)));
  }}
  void add(std::string key, std::vector<std::string> values) {{
    ::rust::Vec<::rust::String> converted;
    converted.reserve(values.size());
    for (auto& value : values) {{
      converted.push_back(::rust::String(std::move(value)));
    }}
    value_->dynamic_attributes_add_string_list(
        ::rust::String(std::move(key)), std::move(converted));
  }}
  void add(std::string key, std::vector<DynamicAttributes> values) {{
    ::rust::Vec<::rust::Box<detail::DynamicAttributesStorage>> converted;
    converted.reserve(values.size());
    for (auto& value : values) {{
      converted.push_back(std::move(value.value_));
    }}
    value_->dynamic_attributes_add_struct_list(
        ::rust::String(std::move(key)), std::move(converted));
  }}
  void add(std::string key, DynamicList value);

 private:
  template <typename T>
  static ::rust::Vec<T> into_rust_vec(std::vector<T> values) {{
    ::rust::Vec<T> output;
    output.reserve(values.size());
    for (auto& value : values) output.push_back(std::move(value));
    return output;
  }}

  ::rust::Box<detail::DynamicAttributesStorage> value_;
  friend struct facade_detail::DynamicAttributesAccess;
  friend class DynamicList;
}};

class DynamicList final {{
 public:
  DynamicList(DynamicList&& other) noexcept
      : value_(std::move(other.value_)) {{
    other.value_ = detail::dynamic_list_empty();
  }}
  DynamicList& operator=(DynamicList&& other) noexcept {{
    if (this != &other) {{
      value_ = std::move(other.value_);
      other.value_ = detail::dynamic_list_empty();
    }}
    return *this;
  }}

  static DynamicList u8(std::vector<std::uint8_t> values) {{
    return DynamicList(detail::dynamic_list_u8(into_rust_vec(std::move(values))));
  }}
  static DynamicList u16(std::vector<std::uint16_t> values) {{
    return DynamicList(detail::dynamic_list_u16(into_rust_vec(std::move(values))));
  }}
  static DynamicList u32(std::vector<std::uint32_t> values) {{
    return DynamicList(detail::dynamic_list_u32(into_rust_vec(std::move(values))));
  }}
  static DynamicList u64(std::vector<std::uint64_t> values) {{
    return DynamicList(detail::dynamic_list_u64(into_rust_vec(std::move(values))));
  }}
  static DynamicList i8(std::vector<std::int8_t> values) {{
    return DynamicList(detail::dynamic_list_i8(into_rust_vec(std::move(values))));
  }}
  static DynamicList i16(std::vector<std::int16_t> values) {{
    return DynamicList(detail::dynamic_list_i16(into_rust_vec(std::move(values))));
  }}
  static DynamicList i32(std::vector<std::int32_t> values) {{
    return DynamicList(detail::dynamic_list_i32(into_rust_vec(std::move(values))));
  }}
  static DynamicList i64(std::vector<std::int64_t> values) {{
    return DynamicList(detail::dynamic_list_i64(into_rust_vec(std::move(values))));
  }}
  static DynamicList f32(std::vector<float> values) {{
    return DynamicList(detail::dynamic_list_f32(into_rust_vec(std::move(values))));
  }}
  static DynamicList f64(std::vector<double> values) {{
    return DynamicList(detail::dynamic_list_f64(into_rust_vec(std::move(values))));
  }}
  static DynamicList string(std::vector<std::string> values) {{
    ::rust::Vec<::rust::String> converted;
    converted.reserve(values.size());
    for (auto& value : values) {{
      converted.push_back(::rust::String(std::move(value)));
    }}
    return DynamicList(detail::dynamic_list_string(std::move(converted)));
  }}
  static DynamicList structures(std::vector<DynamicAttributes> values) {{
    ::rust::Vec<::rust::Box<detail::DynamicAttributesStorage>> converted;
    converted.reserve(values.size());
    for (auto& value : values) {{
      converted.push_back(std::move(value.value_));
    }}
    return DynamicList(detail::dynamic_list_structures(std::move(converted)));
  }}
  static DynamicList list(std::vector<DynamicList> values) {{
    ::rust::Vec<::rust::Box<detail::DynamicListStorage>> converted;
    converted.reserve(values.size());
    for (auto& value : values) {{
      converted.push_back(std::move(value.value_));
    }}
    return DynamicList(detail::dynamic_list_list(std::move(converted)));
  }}

 private:
  explicit DynamicList(::rust::Box<detail::DynamicListStorage> value)
      : value_(std::move(value)) {{}}

  template <typename T>
  static ::rust::Vec<T> into_rust_vec(std::vector<T> values) {{
    ::rust::Vec<T> output;
    output.reserve(values.size());
    for (auto& value : values) output.push_back(std::move(value));
    return output;
  }}

  ::rust::Box<detail::DynamicListStorage> value_;
  friend class DynamicAttributes;
}};

inline void DynamicAttributes::add(std::string key, DynamicList value) {{
  value_->dynamic_attributes_add_list(
      ::rust::String(std::move(key)), std::move(value.value_));
}}

namespace facade_detail {{
struct DynamicAttributesAccess final {{
  static detail::DynamicAttributes take(DynamicAttributes&& value) {{
    detail::DynamicAttributes output{{}};
    output.storage.push_back(std::move(value.value_));
    return output;
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
    for entity in public_entities(schema) {
        let namespace = public_entity_namespace(entity, options);
        output.push_str(&format!("namespace {namespace} {{\n"));
        for event in entity.events() {
            let fields = public_event_fields(entity, event).collect::<Vec<_>>();
            if fields.is_empty() {
                continue;
            }
            output.push_str(&format!(
                "struct {} {{\n",
                to_case(event.name(), Case::Pascal)
            ));
            for field in fields {
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
    for entity in public_entities(schema) {
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
            if public_event_fields(entity, event).next().is_none() {
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
    for entity in public_entities(schema) {
        let namespace = public_entity_namespace(entity, options);
        let parent_namespace = public_entity_parent_namespace(entity, options);
        let name = path_pascal(entity.path());
        let marker = to_case(entity.path().name(), Case::Pascal);
        let entity_type = public_entity_type(entity, options);
        output.push_str(&format!(
            "namespace {parent_namespace} {{ struct {marker} final {{}}; }}\n"
        ));
        if is_fsm(entity) {
            let state_namespace = public_fsm_state_namespace(entity, options);
            output.push_str(&format!("namespace {state_namespace} {{"));
            for event in entity.events() {
                output.push_str(&format!(
                    " struct {} final {{}};",
                    to_case(event.name(), Case::Pascal)
                ));
            }
            output.push_str(" }\n");
        }
        output.push_str(&format!(
            "namespace {namespace} {{ using {name}Id = ::{}::EntityId<{entity_type}>; class {name}Observer; }}\n",
            options.namespace,
        ));
    }
    output.push('\n');
}

fn emit_context_declaration(schema: &Schema, options: &Options, output: &mut String) {
    let namespace = &options.namespace;
    let has_nvtx_source = has_nvtx_source(schema);
    let source_capture = if has_nvtx_source {
        "SourceCapture source_capture = SourceCapture::Enabled"
    } else {
        ""
    };
    let source_capture_suffix = if source_capture.is_empty() {
        String::new()
    } else {
        format!(", {source_capture}")
    };
    let source_capture_enum = if has_nvtx_source {
        "enum class SourceCapture { Enabled, Disabled };\n\n"
    } else {
        ""
    };
    output.push_str(&format!(
        r#"namespace {namespace} {{
{source_capture_enum}class Context final {{
 public:
  Context(Context&&) = default;
  Context& operator=(Context&&) = default;

  static Context none({source_capture});
"#
    ));
    if options.exporters.ndjson {
        output.push_str(&format!(
            "  static Context ndjson(std::string output_dir{source_capture_suffix});\n"
        ));
    }
    if options.exporters.msgpack {
        output.push_str(&format!(
            "  static Context msgpack(std::string output_dir{source_capture_suffix});\n"
        ));
    }
    if options.exporters.postcard {
        output.push_str(&format!(
            "  static Context postcard(std::string output_dir{source_capture_suffix});\n"
        ));
    }
    if options.exporters.collector {
        output.push_str(&format!(
            "  static Context collector(std::string address{source_capture_suffix});\n"
        ));
    }
    output.push_str("\n  Uuid id() const { return inner_->id(); }\n");
    for entity in public_entities(schema) {
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
    for entity in public_entities(schema) {
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
            let fields = public_event_fields(entity, event).collect::<Vec<_>>();
            if fields.is_empty() {
                continue;
            }
            let name = to_case(event.name(), Case::Pascal);
            let public_namespace = public_entity_namespace(entity, options);
            output.push_str(&format!(
                "inline ::{raw_namespace}::{name} {prefix}_to_raw_{name}(::{public_namespace}::{name} value) {{\n  return ::{raw_namespace}::{name}{{\n"
            ));
            for field in fields {
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
    for entity in public_entities(schema) {
        if let Some(fsm) = Fsm::try_from_entity(entity).ok().flatten() {
            emit_fsm_handles(entity, &fsm, options, output);
            continue;
        }
        let namespace = public_entity_namespace(entity, options);
        let raw_namespace = raw_entity_namespace(entity, options);
        let name = path_pascal(entity.path());
        let raw_observer = format!("{name}Observer");
        let raw_handle = format!("{name}Handle");
        let prefix = path_snake(entity.path());
        let entity_type = public_entity_type(entity, options);
        let public_handle = format!("::{base_namespace}::Handle<{entity_type}>");
        output.push_str(&format!(
            "namespace {namespace} {{\nclass {name}Observer final {{\n public:\n  {public_handle} handle() const;\n  {public_handle} handle({name}Id id) const;\n\n private:\n  explicit {name}Observer(::rust::Box<::{raw_namespace}::{raw_observer}> inner) : inner_(std::move(inner)) {{}}\n  ::rust::Box<::{raw_namespace}::{raw_observer}> inner_;\n  friend class ::{base_namespace}::Context;\n}};\n}}  // namespace {namespace}\n\nnamespace {base_namespace} {{\ntemplate <>\nclass Handle<{entity_type}> final {{\n public:\n  Handle(Handle&&) = default;\n  Handle& operator=(Handle&&) = default;\n  ::{namespace}::{name}Id id() const {{ return ::{namespace}::{name}Id(inner_->uuid()); }}\n"
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
                let payload_name = to_case(event.name(), Case::Pascal);
                let payload = format!("::{namespace}::{payload_name}");
                output.push_str(&format!(
                    "  void {method}({payload} data){constness} {{ inner_->{method}(::{base_namespace}::facade_detail::{prefix}_to_raw_{payload_name}(std::move(data))); }}\n"
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
            "\n private:\n  explicit Handle(::rust::Box<::{raw_namespace}::{raw_handle}> inner) : inner_(std::move(inner)) {{}}\n  ::rust::Box<::{raw_namespace}::{raw_handle}> inner_;\n  friend struct facade_detail::HandleAccess;\n}};\n}}  // namespace {base_namespace}\n\nnamespace {namespace} {{\ninline {public_handle} {name}Observer::handle() const {{\n  return ::{base_namespace}::facade_detail::HandleAccess::make<{public_handle}>(inner_->handle());\n}}\ninline {public_handle} {name}Observer::handle({name}Id id) const {{\n  return ::{base_namespace}::facade_detail::HandleAccess::make<{public_handle}>(inner_->handle_with_id(id.raw()));\n}}\n}}  // namespace {namespace}\n\n"
        ));
    }
}

fn emit_fsm_handles(entity: &Entity, fsm: &Fsm, options: &Options, output: &mut String) {
    let namespace = public_entity_namespace(entity, options);
    let raw_namespace = raw_entity_namespace(entity, options);
    let base_namespace = &options.namespace;
    let name = path_pascal(entity.path());
    let raw_observer = format!("{name}Observer");
    let raw_handle = format!("{name}Handle");
    let prefix = path_snake(entity.path());
    let entity_type = public_entity_type(entity, options);
    let initial_handle = public_fsm_handle_type(entity, None, options);

    output.push_str(&format!(
        "namespace {namespace} {{\nclass {name}Observer final {{\n public:\n  {initial_handle} handle() const;\n  {initial_handle} handle({name}Id id) const;\n\n private:\n  explicit {name}Observer(::rust::Box<::{raw_namespace}::{raw_observer}> inner) : inner_(std::move(inner)) {{}}\n  ::rust::Box<::{raw_namespace}::{raw_observer}> inner_;\n  friend class ::{base_namespace}::Context;\n}};\n}}  // namespace {namespace}\n\n"
    ));

    output.push_str(&format!("namespace {base_namespace} {{\n"));
    for state in std::iter::once(None).chain(entity.events().map(Some)) {
        let state_type = state.map_or_else(
            || format!("::{base_namespace}::facade_detail::InitialFsmState"),
            |event| public_fsm_state_type(entity, event, options),
        );
        output.push_str(&format!(
            "template <>\nclass FsmHandle<{entity_type}, {state_type}> final {{\n public:\n  FsmHandle(FsmHandle&&) = default;\n  FsmHandle& operator=(FsmHandle&&) = default;\n  ::{namespace}::{name}Id id() const {{ return ::{namespace}::{name}Id(inner_->uuid()); }}\n"
        ));
        if let Some(state) = state {
            for transition in fsm
                .transitions()
                .iter()
                .filter(|transition| transition.source() == state.name())
            {
                let target = entity
                    .events()
                    .find(|event| event.name() == transition.target())
                    .expect("validated FSM transition target");
                emit_fsm_method_declaration(entity, target, options, output);
            }
        } else {
            let initial = entity
                .events()
                .find(|event| event.name() == fsm.initial_state())
                .expect("validated FSM initial state");
            emit_fsm_method_declaration(entity, initial, options, output);
        }
        output.push_str(&format!(
            "\n private:\n  explicit FsmHandle(::rust::Box<::{raw_namespace}::{raw_handle}> inner) : inner_(std::move(inner)) {{}}\n  ::rust::Box<::{raw_namespace}::{raw_handle}> inner_;\n  friend struct facade_detail::HandleAccess;\n}};\n\n"
        ));
    }

    let initial = entity
        .events()
        .find(|event| event.name() == fsm.initial_state())
        .expect("validated FSM initial state");
    emit_fsm_method_definition(entity, &initial_handle, initial, options, &prefix, output);
    for transition in fsm.transitions() {
        let source_event = entity
            .events()
            .find(|event| event.name() == transition.source())
            .expect("validated FSM transition source");
        let source = public_fsm_handle_type(entity, Some(source_event), options);
        let target = entity
            .events()
            .find(|event| event.name() == transition.target())
            .expect("validated FSM transition target");
        emit_fsm_method_definition(entity, &source, target, options, &prefix, output);
    }
    output.push_str(&format!("}}  // namespace {base_namespace}\n\n"));

    output.push_str(&format!(
        "namespace {namespace} {{\ninline {initial_handle} {name}Observer::handle() const {{\n  return ::{base_namespace}::facade_detail::HandleAccess::make<{initial_handle}>(inner_->handle());\n}}\ninline {initial_handle} {name}Observer::handle({name}Id id) const {{\n  return ::{base_namespace}::facade_detail::HandleAccess::make<{initial_handle}>(inner_->handle_with_id(id.raw()));\n}}\n}}  // namespace {namespace}\n\n"
    ));
}

fn emit_fsm_method_declaration(
    entity: &Entity,
    event: &Event,
    options: &Options,
    output: &mut String,
) {
    let method = cxx_safe(&to_case(event.name(), Case::Snake));
    let target = public_fsm_handle_type(entity, Some(event), options);
    if public_event_fields(entity, event).next().is_none() {
        output.push_str(&format!("  {target} {method}() &&;\n"));
    } else {
        let payload = format!(
            "::{}::{}",
            public_entity_namespace(entity, options),
            to_case(event.name(), Case::Pascal)
        );
        output.push_str(&format!("  {target} {method}({payload} data) &&;\n"));
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_fsm_method_definition(
    entity: &Entity,
    source: &str,
    event: &Event,
    options: &Options,
    prefix: &str,
    output: &mut String,
) {
    let base_namespace = &options.namespace;
    let method = cxx_safe(&to_case(event.name(), Case::Snake));
    let target = public_fsm_handle_type(entity, Some(event), options);
    let source = source
        .strip_prefix(&format!("::{base_namespace}::"))
        .expect("FSM handle belongs to the base namespace");
    if public_event_fields(entity, event).next().is_none() {
        output.push_str(&format!(
            "inline {target} {source}::{method}() && {{\n  inner_->{method}();\n  return facade_detail::HandleAccess::make<{target}>(std::move(inner_));\n}}\n"
        ));
    } else {
        let payload_name = to_case(event.name(), Case::Pascal);
        let payload = format!(
            "::{}::{payload_name}",
            public_entity_namespace(entity, options)
        );
        output.push_str(&format!(
            "inline {target} {source}::{method}({payload} data) && {{\n  inner_->{method}(::{base_namespace}::facade_detail::{prefix}_to_raw_{payload_name}(std::move(data)));\n  return facade_detail::HandleAccess::make<{target}>(std::move(inner_));\n}}\n"
        ));
    }
}

fn is_fsm(entity: &Entity) -> bool {
    Fsm::try_from_entity(entity).ok().flatten().is_some()
}

fn public_event_fields<'a>(entity: &Entity, event: &'a Event) -> impl Iterator<Item = &'a Field> {
    let is_fsm = is_fsm(entity);
    event
        .fields()
        .filter(move |field| !is_fsm || field.name() != SEQUENCE_FIELD_NAME)
}

fn emit_context_methods(schema: &Schema, options: &Options, output: &mut String) {
    let namespace = &options.namespace;
    let has_nvtx_source = has_nvtx_source(schema);
    let source_capture_parameter = if has_nvtx_source {
        "SourceCapture source_capture"
    } else {
        ""
    };
    let source_capture_parameter_suffix = if has_nvtx_source {
        ", SourceCapture source_capture"
    } else {
        ""
    };
    let source_capture_argument = if has_nvtx_source {
        ", source_capture == SourceCapture::Enabled"
    } else {
        ""
    };
    output.push_str(&format!(
        r#"namespace {namespace} {{
inline Context Context::none({source_capture_parameter}) {{
  return Context(detail::create_context(detail::ExporterOptions::none(){source_capture_argument}));
}}
"#
    ));
    if options.exporters.ndjson {
        output.push_str(&format!(
            "inline Context Context::ndjson(std::string output_dir{}) {{\n  return Context(detail::create_context(\n      detail::ExporterOptions::ndjson(::rust::String(std::move(output_dir))){}));\n}}\n",
            source_capture_parameter_suffix,
            source_capture_argument,
        ));
    }
    if options.exporters.msgpack {
        output.push_str(&format!(
            "inline Context Context::msgpack(std::string output_dir{}) {{\n  return Context(detail::create_context(\n      detail::ExporterOptions::msgpack(::rust::String(std::move(output_dir))){}));\n}}\n",
            source_capture_parameter_suffix,
            source_capture_argument,
        ));
    }
    if options.exporters.postcard {
        output.push_str(&format!(
            "inline Context Context::postcard(std::string output_dir{}) {{\n  return Context(detail::create_context(\n      detail::ExporterOptions::postcard(::rust::String(std::move(output_dir))){}));\n}}\n",
            source_capture_parameter_suffix,
            source_capture_argument,
        ));
    }
    if options.exporters.collector {
        output.push_str(&format!(
            "inline Context Context::collector(std::string address{}) {{\n  return Context(detail::create_context(\n      detail::ExporterOptions::collector(::rust::String(std::move(address))){}));\n}}\n",
            source_capture_parameter_suffix,
            source_capture_argument,
        ));
    }
    for entity in public_entities(schema) {
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
                "[] ({} input) {{ ::rust::Vec<{raw_item}> output; output.reserve(input.size()); for (auto&& item : input) {{ output.push_back({converted}); }} return output; }}({expression})",
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
    for entity in public_entities(schema) {
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

fn public_entity_parent_namespace(entity: &Entity, options: &Options) -> String {
    std::iter::once(options.namespace.clone())
        .chain(
            entity
                .path()
                .namespace()
                .iter()
                .map(|part| cxx_safe(&to_case(part, Case::Snake))),
        )
        .collect::<Vec<_>>()
        .join("::")
}

fn public_entity_type(entity: &Entity, options: &Options) -> String {
    format!(
        "::{}::{}",
        public_entity_parent_namespace(entity, options),
        to_case(entity.path().name(), Case::Pascal)
    )
}

fn public_fsm_state_namespace(entity: &Entity, options: &Options) -> String {
    format!(
        "{}::{}_state",
        public_entity_parent_namespace(entity, options),
        cxx_safe(&to_case(entity.path().name(), Case::Snake))
    )
}

fn public_fsm_state_type(entity: &Entity, event: &Event, options: &Options) -> String {
    format!(
        "::{}::{}",
        public_fsm_state_namespace(entity, options),
        to_case(event.name(), Case::Pascal)
    )
}

fn public_fsm_handle_type(entity: &Entity, state: Option<&Event>, options: &Options) -> String {
    let entity_type = public_entity_type(entity, options);
    match state {
        Some(state) => format!(
            "::{}::FsmHandle<{entity_type}, {}>",
            options.namespace,
            public_fsm_state_type(entity, state, options)
        ),
        None => format!("::{}::FsmHandle<{entity_type}>", options.namespace),
    }
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
