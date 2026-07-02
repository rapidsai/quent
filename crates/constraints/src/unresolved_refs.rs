// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{
    DataType,
    visitor::{Cursor, Element, Visitor},
};

/// Reports every internal reference that does not resolve.
///
/// Note that constraints adding internal references to schema elements through
/// annotations are responsible for validating those internal references
/// themselves.
#[derive(Default)]
pub struct UnresolvedReferences {
    found: Vec<String>,
}
impl Visitor for UnresolvedReferences {
    type Output = Vec<String>;
    fn visit(&mut self, cursor: &Cursor) {
        if let Element::DataType(DataType::Record(name)) = cursor.current()
            && cursor.root().record(name).is_none()
        {
            self.found.push(format!("{cursor}: {name}"));
        }
    }
    fn finish(self) -> Self::Output {
        self.found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_schema::Schema;
    use quent_schema::test_utils::{entity, event, field, ident, record, schema};

    fn unresolved(schema: &Schema) -> Vec<String> {
        schema.walk(UnresolvedReferences::default())
    }

    #[test]
    fn consistent_schema_passes() {
        let s = schema(
            "S",
            vec![entity(
                "E",
                vec![event("Ev", vec![field("f", DataType::U64)])],
            )],
            vec![record("R", vec![field("rf", DataType::Record(ident("R")))])],
        );
        assert!(unresolved(&s).is_empty());
    }

    #[test]
    fn unresolved_record_reference_is_reported() {
        let s = schema(
            "S",
            vec![entity(
                "E",
                vec![event(
                    "Ev",
                    vec![field("f", DataType::Record(ident("ghost")))],
                )],
            )],
            vec![],
        );
        assert_eq!(
            unresolved(&s),
            vec!["S.E.Ev.f.Record(ghost): ghost".to_string()]
        );
    }

    #[test]
    fn resolved_record_reference_passes() {
        let s = schema(
            "S",
            vec![entity(
                "E",
                vec![event("Ev", vec![field("f", DataType::Record(ident("R")))])],
            )],
            vec![record("R", vec![])],
        );
        assert!(unresolved(&s).is_empty());
    }

    #[test]
    fn references_into_nested_data_types_are_checked() {
        let ty = DataType::Option(Box::new(DataType::List(Box::new(DataType::Record(ident(
            "ghost",
        ))))));
        let s = schema(
            "S",
            vec![entity("E", vec![event("Ev", vec![field("f", ty)])])],
            vec![],
        );
        assert!(unresolved(&s).iter().any(|r| r.contains("ghost")));
    }
}
