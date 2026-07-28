// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::visitor::{Cursor, Element, Visitor};
use std::collections::HashSet;

/// Reports paths declared by both a record and an entity.
#[derive(Default)]
pub(crate) struct DuplicateTypePaths {
    found: Vec<String>,
}

impl Visitor for DuplicateTypePaths {
    type Output = Vec<String>;

    fn visit(&mut self, cursor: &Cursor) {
        let Element::Schema(schema) = cursor.current() else {
            return;
        };
        let records: HashSet<_> = schema.records().map(|record| record.path()).collect();
        self.found.extend(
            schema
                .entities()
                .filter(|entity| records.contains(entity.path()))
                .map(|entity| entity.path().to_string()),
        );
    }

    fn finish(self) -> Self::Output {
        self.found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_schema::test_utils::{entity, event, record, unchecked_schema};

    #[test]
    fn reports_a_path_used_by_both_type_kinds() {
        let schema = unchecked_schema(
            "Schema",
            [
                entity("Shared", [event("event", [])]),
                entity("Foo::Q", [event("event", [])]),
                entity("Bar::Q", [event("event", [])]),
            ],
            [
                record("Shared", []),
                record("Foo::R", []),
                record("Bar::R", []),
            ],
        );

        assert_eq!(
            schema.walk(DuplicateTypePaths::default()),
            ["Shared".to_string()]
        );
    }
}
