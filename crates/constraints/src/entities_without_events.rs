// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::visitor::{Cursor, Element, Visitor};

/// Reports every entity that declares no events.
#[derive(Default)]
pub(crate) struct EntitiesWithoutEvents {
    entities: Vec<String>,
}

impl Visitor for EntitiesWithoutEvents {
    type Output = Vec<String>;

    fn visit(&mut self, cursor: &Cursor) {
        if let Element::Entity(entity) = cursor.current()
            && entity.events().len() == 0
        {
            self.entities.push(entity.path().to_string());
        }
    }

    fn finish(self) -> Self::Output {
        self.entities
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_schema::Schema;
    use quent_schema::test_utils::{eventless_entity, schema};

    fn walk(schema: &Schema) -> Vec<String> {
        schema.walk(EntitiesWithoutEvents::default())
    }

    #[test]
    fn entities_without_events_are_reported() {
        let schema = schema(
            "S",
            vec![eventless_entity("A"), eventless_entity("B")],
            vec![],
        );
        assert_eq!(walk(&schema), vec!["A".to_string(), "B".to_string()]);
    }
}
