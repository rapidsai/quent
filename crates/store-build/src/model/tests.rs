// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{
    DataType,
    builder::AnnotationsBuilder,
    test_utils::{entity, event, field, schema},
};

use crate::{GenerateError, Options, generate_str};

#[test]
fn rejects_model_store_name_conflicts() {
    let result = generate_str(
        &schema("Demo", [entity("DemoStore", [event("created", [])])], []),
        &Options::default(),
    );
    assert!(
        matches!(result, Err(GenerateError::EntityEventsNameConflict { .. })),
        "expected a name conflict, got {result:?}"
    );
}

#[test]
fn rejects_tree_traversal_name_conflicts() {
    for name in ["DemoTree", "DemoTreeVisitor"] {
        let parent = DataType::EntityRef {
            data: None,
            annotations: AnnotationsBuilder::new()
                .with_constraint("quent.ref-target.v0.1.0", Some(name.to_owned()))
                .with_constraint("quent.ref-tree.v0.1.0", None)
                .build()
                .unwrap(),
        };
        let result = generate_str(
            &schema(
                "Demo",
                [
                    entity(name, [event("created", [])]),
                    entity("Child", [event("created", [field("parent", parent)])]),
                ],
                [],
            ),
            &Options::default(),
        );
        assert!(
            matches!(result, Err(GenerateError::EntityEventsNameConflict { name: conflict, .. }) if conflict == name)
        );
    }
}

#[test]
fn model_store_handles_namespaced_entities_and_field_name_collisions() {
    let source = generate_str(
        &schema(
            "Demo",
            [
                entity("Server", [event("created", [])]),
                entity("Left::Worker", [event("created", [])]),
                entity("Right::Worker", [event("created", [])]),
                entity("type", [event("created", [])]),
                entity("FooBar::Worker", [event("created", [])]),
                entity("Foo::BarWorker", [event("created", [])]),
            ],
            [],
        ),
        &Options::default(),
    )
    .unwrap();
    let file = syn::parse_file(&source).unwrap();
    let layout = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == "DemoStore" => Some(item),
            _ => None,
        })
        .unwrap();
    let mut fields = layout
        .fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().to_string())
        .collect::<Vec<_>>();
    fields.sort();
    assert_eq!(
        fields,
        [
            "foo_bar_worker",
            "foo_bar_worker_2",
            "left_worker",
            "r#type",
            "right_worker",
            "server"
        ]
    );
}
