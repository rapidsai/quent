// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint;
use quent_ref_target::{RefTargetConstraint, RefTargetError};
use quent_schema::{
    DataType,
    builder::{AnnotationsBuilder, SchemaBuilder},
    test_utils::{entity, event, field, record, schema},
};

use crate::{Error, Options, generate_str};

/// Checks that generation rejects schema references to entities that do not
/// exist.
#[test]
fn rejects_unknown_reference_targets_before_generating_code() {
    let annotations = AnnotationsBuilder::new()
        .with_constraint(RefTargetConstraint::NAME, Some("Missing".to_owned()))
        .build()
        .unwrap();
    let schema = schema(
        "Demo",
        [entity(
            "Task",
            [event(
                "created",
                [field(
                    "parent",
                    DataType::EntityRef {
                        data: None,
                        annotations,
                    },
                )],
            )],
        )],
        [],
    );

    assert!(matches!(
        generate_str(&schema, &Options::default(), &runtime()),
        Err(Error::InvalidRefTarget(
            RefTargetError::UnknownTarget { .. }
        ))
    ));
}

pub(crate) fn runtime() -> crate::Runtime {
    crate::Runtime {
        events: syn::parse_quote!(::quent_events),
        entity_marker: syn::parse_quote!(::quent_events::EntityMarker),
        serde: None,
    }
}

#[test]
fn built_in_derive_path_spellings_are_deduplicated() {
    let schema = SchemaBuilder::try_new("Demo")
        .unwrap()
        .with_record(record("Meta", []))
        .with_entity(entity("Query", [event("created", [])]))
        .build()
        .unwrap();
    let opts = Options {
        debug: true,
        serde: true,
        event_derives: &[
            "Debug",
            "std::fmt::Debug",
            "::core::fmt::Debug",
            "serde::Serialize",
            "::serde::Serialize",
        ],
        record_derives: &[
            "Debug",
            "core::fmt::Debug",
            "::std::fmt::Debug",
            "serde::Deserialize",
            "::serde::Deserialize",
        ],
        ..Options::default()
    };

    let source = generate_str(&schema, &opts, &runtime()).unwrap();

    let derives = "#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]";
    assert_eq!(source.matches(derives).count(), 2);
}
