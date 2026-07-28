// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Reference tests: `ref:` and `scope-ref:` emit the right constraints and are
//! validated against the schema.

use quent_schema::test_utils::{ident, path};
use quent_schema::{Annotations, Cardinality, DataType, Schema};
use quent_yaml::parse_from_str;

const REF_TARGET: &str = "quent.ref-target.v0.1.0";
const REF_TREE: &str = "quent.ref-tree.v0.1.0";

fn schema_of(src: &str) -> Schema {
    parse_from_str(src, None).expect("parses").schema
}

fn cardinality(schema: &Schema, entity: &str, event: &str) -> Cardinality {
    schema
        .entity(&path(entity))
        .unwrap()
        .event(&ident(event))
        .unwrap()
        .cardinality()
}

/// The annotations on `entity.event.field`, which must be an entity reference.
fn ref_annotations<'s>(
    schema: &'s Schema,
    entity: &str,
    event: &str,
    field: &str,
) -> &'s Annotations {
    let field = schema
        .entity(&path(entity))
        .unwrap()
        .event(&ident(event))
        .unwrap()
        .field(&ident(field))
        .unwrap();
    let DataType::EntityRef { annotations, .. } = field.ty() else {
        panic!("expected an entity ref, got {:?}", field.ty());
    };
    annotations
}

#[test]
fn ref_emits_target_only() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Cluster:
    events:
      up: {}
  Engine:
    events:
      started:
        attributes:
          cluster: { ref: Cluster }
",
    );
    assert_eq!(cardinality(&schema, "Engine", "started"), Cardinality::Once);
    let anns = ref_annotations(&schema, "Engine", "started", "cluster");
    assert_eq!(anns.constraint(REF_TARGET).unwrap().data(), Some("Cluster"));
    assert!(!anns.has_constraint(REF_TREE));
}

#[test]
fn ref_can_carry_data() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Cluster:
    events:
      up: {}
  Engine:
    events:
      started:
        multi: true
        attributes:
          cluster:
            ref: Cluster
            data: u64
",
    );
    assert_eq!(
        cardinality(&schema, "Engine", "started"),
        Cardinality::Multi
    );
    let field = schema
        .entity(&path("Engine"))
        .unwrap()
        .event(&ident("started"))
        .unwrap()
        .field(&ident("cluster"))
        .unwrap();
    let DataType::EntityRef { data, annotations } = field.ty() else {
        panic!("expected an entity ref");
    };
    assert_eq!(data.as_deref(), Some(&DataType::U64));
    assert_eq!(
        annotations.constraint(REF_TARGET).unwrap().data(),
        Some("Cluster")
    );
}

#[test]
fn scope_emits_target_and_tree() {
    // Cluster is the root (no scope), Engine is scoped by it — a valid tree.
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  Cluster:
    events:
      up: {}
  Engine:
    events:
      started:
        attributes:
          parent:
            scope-ref: Cluster
            data: u64
",
    );
    let field = schema
        .entity(&path("Engine"))
        .unwrap()
        .event(&ident("started"))
        .unwrap()
        .field(&ident("parent"))
        .unwrap();
    let DataType::EntityRef { data, annotations } = field.ty() else {
        panic!("expected an entity ref");
    };
    assert_eq!(data.as_deref(), Some(&DataType::U64));
    assert_eq!(
        annotations.constraint(REF_TARGET).unwrap().data(),
        Some("Cluster")
    );
    assert!(annotations.has_constraint(REF_TREE));
}
