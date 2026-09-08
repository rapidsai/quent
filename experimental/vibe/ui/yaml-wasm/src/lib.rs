// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;
use wasm_bindgen::prelude::*;

const NAMESPACE_SEPARATOR: &str = "QuentNamespaceSeparator";

/// Parse YAML and return the validated schema as JSON.
///
/// Parser and validation diagnostics are returned as a rejected JavaScript value.
#[wasm_bindgen]
pub fn parse_schema_json(source: &str) -> Result<String, JsValue> {
    parse_schema_value(source)
        .and_then(|schema| serde_json::to_string(&schema).map_err(|error| error.to_string()))
        .map_err(|error| JsValue::from_str(&error))
}

fn parse_schema_value(source: &str) -> Result<Value, String> {
    let encoded = source.replace("::", NAMESPACE_SEPARATOR);
    let parsed = quent_yaml::parse_from_str(encoded, Some("editor.yaml"))
        .map_err(|error| error.to_string())?;
    let mut schema = serde_json::to_value(parsed.schema).map_err(|error| error.to_string())?;
    restore_namespaces(&mut schema);
    Ok(schema)
}

fn restore_namespaces(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(restore_namespaces),
        Value::Object(values) => {
            let restored_path = values
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| {
                    values.contains_key("namespace") && name.contains(NAMESPACE_SEPARATOR)
                })
                .map(|name| {
                    let mut segments = name
                        .split(NAMESPACE_SEPARATOR)
                        .map(str::to_owned)
                        .collect::<Vec<_>>();
                    let name = segments.pop().expect("a namespaced path has a name");
                    (segments, name)
                });
            if let Some((namespace, name)) = restored_path {
                values.insert("name".to_owned(), Value::String(name));
                values.insert(
                    "namespace".to_owned(),
                    Value::Array(namespace.into_iter().map(Value::String).collect()),
                );
            }
            values.values_mut().for_each(restore_namespaces);
        }
        Value::String(value) => {
            *value = value.replace(NAMESPACE_SEPARATOR, "::");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_namespaced_declarations_and_references() {
        let schema = parse_schema_value(
            "\
quent: alpha
model: Namespaced
entities:
  Root::Parent:
    events:
      started: {}
  Child::Worker:
    events:
      started:
        attributes:
          parent: { scope-ref: Root::Parent }
",
        )
        .expect("schema parses");

        let paths = schema["entities"]
            .as_array()
            .expect("schema entities")
            .iter()
            .map(|entry| &entry[0])
            .collect::<Vec<_>>();
        assert!(paths.contains(&&serde_json::json!({
            "namespace": ["Root"],
            "name": "Parent",
        })));
        assert!(paths.contains(&&serde_json::json!({
            "namespace": ["Child"],
            "name": "Worker",
        })));
        let json = schema.to_string();
        assert!(json.contains("Root::Parent"));
        assert!(!json.contains(NAMESPACE_SEPARATOR));
    }
}
