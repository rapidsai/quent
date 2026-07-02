// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

use petgraph::{algo::tarjan_scc, graphmap::DiGraphMap};
use quent_schema::{
    DataType, Identifier,
    visitor::{Cursor, Element, Visitor},
};
use rustc_hash::FxHashMap;

/// Reports every record that is recursive, i.e. references itself in any of its
/// (nested) field types.
#[derive(Default)]
pub(crate) struct RecursiveRecords {
    // Record name -> the records its fields reference.
    edges: FxHashMap<Identifier, Vec<Identifier>>,
}

impl Visitor for RecursiveRecords {
    type Output = Vec<String>;

    fn visit(&mut self, cursor: &Cursor) {
        // If the current cursor is on a datatype with a record, and the full
        // path starts at a record, then we have a record referencing a record.
        // This is where loops could be created.
        if let Element::DataType(DataType::Record(target)) = cursor.current()
            && let [_, Element::Record(source), ..] = cursor.elements()
        {
            self.edges
                .entry(source.name().clone())
                .or_default()
                .push(target.clone());
        }
    }

    fn finish(self) -> Self::Output {
        // Construct a graph of all records referencing all other records.
        let graph: DiGraphMap<&Identifier, ()> = self
            .edges
            .iter()
            .flat_map(|(source, targets)| targets.iter().map(move |target| (source, target)))
            .collect();

        let mut recursive: BTreeSet<&Identifier> = BTreeSet::new();
        // Records in a strongly connected component of more than one node are
        // mutually recursive.
        for scc in tarjan_scc(&graph) {
            if scc.len() > 1 {
                recursive.extend(scc);
            }
        }
        // Immediate self-loops are single node components so need to be checked
        // separately
        for (source, targets) in &self.edges {
            if targets.contains(source) {
                recursive.insert(source);
            }
        }
        recursive.into_iter().map(ToString::to_string).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quent_schema::Schema;
    use quent_schema::test_utils::{entity, event, field, ident, record, schema};

    fn walk(schema: &Schema) -> Vec<String> {
        schema.walk(RecursiveRecords::default())
    }

    #[test]
    fn non_recursive_records_pass() {
        // S references R, R is a leaf: no cycle.
        let s = schema(
            "S",
            vec![],
            vec![
                record("S", vec![field("f", DataType::Record(ident("R")))]),
                record("R", vec![field("g", DataType::U64)]),
            ],
        );
        assert!(walk(&s).is_empty());
    }

    #[test]
    fn self_recursive_record_is_reported() {
        let s = schema(
            "S",
            vec![],
            vec![record("R", vec![field("f", DataType::Record(ident("R")))])],
        );
        assert_eq!(walk(&s), vec!["R".to_string()]);
    }

    #[test]
    fn mutually_recursive_records_are_reported() {
        let s = schema(
            "S",
            vec![],
            vec![
                record("A", vec![field("f", DataType::Record(ident("B")))]),
                record("B", vec![field("g", DataType::Record(ident("A")))]),
            ],
        );
        assert_eq!(walk(&s), vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn recursion_through_option_and_list_wrappers_is_reported() {
        let cases = [
            (
                "A",
                DataType::Option(Box::new(DataType::Record(ident("A")))),
            ),
            ("B", DataType::List(Box::new(DataType::Record(ident("B"))))),
            (
                "C",
                DataType::List(Box::new(DataType::Option(Box::new(DataType::Record(
                    ident("C"),
                ))))),
            ),
        ];
        for (name, ty) in cases {
            let s = schema("S", vec![], vec![record(name, vec![field("f", ty)])]);
            assert_eq!(walk(&s), vec![name.to_string()]);
        }
    }

    #[test]
    fn entity_record_reference_is_not_recursion() {
        let s = schema(
            "S",
            vec![entity(
                "E",
                vec![event("Ev", vec![field("f", DataType::Record(ident("R")))])],
            )],
            vec![record("R", vec![field("g", DataType::U64)])],
        );
        assert!(walk(&s).is_empty());
    }
}
