// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::{Constraint, validate};
use quent_schema::{
    Annotations, Cardinality, DataType, Field, Schema,
    builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder, SchemaBuilder},
    test_utils::{
        self, entity, event, eventless_entity, field, ident, record, record_type, schema,
    },
    visitor::{Cursor, Visitor},
};

/// Annotations carrying the constraint `name` without data.
fn constraint(name: &str) -> Annotations {
    AnnotationsBuilder::new()
        .with_constraint(name, None)
        .build()
        .unwrap()
}

fn empty_schema() -> Schema {
    test_utils::schema("TestSchema", vec![], vec![])
}

fn ghost_ref_schema() -> Schema {
    schema(
        "S",
        vec![entity(
            "E",
            vec![event("Ev", vec![field("f", record_type("ghost"))])],
        )],
        vec![],
    )
}

// A constraint that finds no violations.
#[derive(Default)]
struct NoopA;
impl Visitor for NoopA {
    type Output = Result<(), Box<dyn std::error::Error>>;
    fn visit(&mut self, _cursor: &Cursor) {}
    fn finish(self) -> Self::Output {
        Ok(())
    }
}
impl Constraint for NoopA {
    const NAME: &'static str = "a";
}

#[derive(Default)]
struct NoopB;
impl Visitor for NoopB {
    type Output = Result<(), Box<dyn std::error::Error>>;
    fn visit(&mut self, _cursor: &Cursor) {}
    fn finish(self) -> Self::Output {
        Ok(())
    }
}
impl Constraint for NoopB {
    const NAME: &'static str = "b";
}

// A minimal error type for a failing constraint.
#[derive(Debug)]
struct Boom(&'static str);
impl std::fmt::Display for Boom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for Boom {}

// A constraint that always fails.
#[derive(Default)]
struct Failing;
impl Visitor for Failing {
    type Output = Result<(), Box<dyn std::error::Error>>;
    fn visit(&mut self, _cursor: &Cursor) {}
    fn finish(self) -> Self::Output {
        Err(Box::new(Boom("boom")))
    }
}
impl Constraint for Failing {
    const NAME: &'static str = "a";
}

#[test]
fn passing_constraint_on_empty_schema() {
    let report = validate::<(NoopA,)>(&empty_schema());
    assert!(report.unregistered_constraints.is_empty());
    assert!(report.results.0.is_ok());
}

#[test]
fn constraint_without_validator_is_unregistered() {
    let schema = SchemaBuilder::new(ident("TestSchema"))
        .with_annotations(constraint("unknown"))
        .build()
        .unwrap();
    let report = validate::<(NoopA,)>(&schema);
    assert_eq!(report.unregistered_constraints.len(), 1);
    assert!(
        report
            .unregistered_constraints
            .iter()
            .any(|c| c == "unknown")
    );
    assert!(report.results.0.is_ok());
}

#[test]
fn metadata_is_never_validated() {
    let schema = SchemaBuilder::new(ident("TestSchema"))
        .with_annotations(
            AnnotationsBuilder::new()
                .with_metadata("not_validated", None)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let report = validate::<(NoopA,)>(&schema);
    assert!(report.unregistered_constraints.is_empty());
}

#[test]
fn unregistered_constraint_is_reported_once() {
    let unknown = || constraint("unknown");
    let field = Field::new(ident("ef"), DataType::U64, unknown());
    let event = EventBuilder::new(ident("Ev"), Cardinality::Once)
        .with_field(field)
        .with_annotations(unknown())
        .build()
        .unwrap();
    let entity = EntityBuilder::new(ident("E"))
        .with_event(event)
        .with_annotations(unknown())
        .build()
        .unwrap();
    let record_field = Field::new(ident("rf"), DataType::U64, unknown());
    let record = RecordBuilder::new(ident("R"))
        .with_field(record_field)
        .with_annotations(unknown())
        .build()
        .unwrap();
    let schema = SchemaBuilder::new(ident("S"))
        .with_entity(entity)
        .with_record(record)
        .with_annotations(unknown())
        .build()
        .unwrap();
    let report = validate::<(NoopA,)>(&schema);
    // The same name used at six sites is deduplicated to a single entry.
    assert_eq!(
        report
            .unregistered_constraints
            .into_iter()
            .collect::<Vec<_>>(),
        vec!["unknown".to_string()]
    );
}

#[test]
fn constraint_failure_is_reported_per_constraint() {
    let report = validate::<(Failing, NoopB)>(&empty_schema());
    assert!(report.results.0.is_err());
    assert_eq!(report.results.0.as_ref().unwrap_err().to_string(), "boom");
    assert!(report.results.1.is_ok());
}

#[test]
fn unregistered_and_failure_aggregate() {
    let schema = SchemaBuilder::new(ident("TestSchema"))
        .with_annotations(constraint("unknown"))
        .build()
        .unwrap();
    let report = validate::<(Failing,)>(&schema);
    assert!(
        report
            .unregistered_constraints
            .iter()
            .any(|c| c == "unknown")
    );
    assert!(report.results.0.is_err());
}

#[test]
fn consistency_is_always_checked_in_the_same_walk() {
    // A field referencing a record that does not exist.
    let schema = ghost_ref_schema();
    let report = validate::<(NoopA,)>(&schema);
    // The constraint passed, but the schema is internally inconsistent: a
    // reference to a record that does not exist.
    assert!(report.results.0.is_ok());
    assert_eq!(
        report
            .base_constraints
            .unwrap_err()
            .invalid_references
            .len(),
        1
    );
}

#[test]
fn validates_consistency_with_no_constraints() {
    let schema = ghost_ref_schema();
    // No constraints, just the always-on consistency checks.
    let report = validate::<()>(&schema);
    assert_eq!(report.results, ());
    assert_eq!(
        report
            .base_constraints
            .unwrap_err()
            .invalid_references
            .len(),
        1
    );

    let clean = validate::<()>(&empty_schema());
    assert!(clean.base_constraints.is_ok());
}

#[test]
fn entity_without_events_fails_base_validation() {
    let schema = schema("S", vec![eventless_entity("E")], vec![]);
    let error = validate::<()>(&schema).base_constraints.unwrap_err();

    assert_eq!(error.entities_without_events, vec!["E".to_string()]);
    assert!(error.to_string().contains("  - E"));
}

#[test]
fn empty_record_passes_base_validation() {
    let schema = schema("S", vec![], vec![record("Empty", vec![])]);
    assert!(validate::<()>(&schema).base_constraints.is_ok());
}
