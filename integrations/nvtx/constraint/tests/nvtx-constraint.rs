// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::{Constraint as _, validate};
use quent_nvtx::{NvtxConstraint, NvtxEntityBuilder, canonical_records, nvtx_path};
use quent_ref_target::RefTargetConstraint;
use quent_ref_tree::RefTreeConstraint;
use quent_schema::{
    Cardinality, DataType, Entity, Event, Record,
    builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder},
    test_utils::{event, field, ident, path, record_type, unchecked_schema},
};

fn sink() -> Entity {
    NvtxEntityBuilder::new().build().unwrap()
}

fn errors(sink: Entity, records: Vec<Record>) -> String {
    validate::<(NvtxConstraint,)>(&unchecked_schema("Application", [sink], records))
        .results
        .0
        .unwrap_err()
        .to_string()
}

fn replace_event(sink: &Entity, name: &str, replacement: Option<Event>) -> Entity {
    EntityBuilder::new(sink.path().clone())
        .with_annotations(sink.annotations().clone())
        .with_events(
            sink.events()
                .filter(|event| event.name() != &ident(name))
                .cloned(),
        )
        .with_events(replacement)
        .build()
        .unwrap()
}

#[test]
fn canonical_sink_is_valid_without_an_application_process_type() {
    let schema = unchecked_schema("Application", [sink()], canonical_records());
    let report = validate::<(NvtxConstraint, RefTargetConstraint, RefTreeConstraint)>(&schema);
    assert!(report.base_constraints.is_ok(), "{report:?}");
    assert!(report.unregistered_constraints.is_empty(), "{report:?}");
    let (nvtx, target, tree) = report.results;
    nvtx.unwrap();
    target.unwrap();
    tree.unwrap();
}

#[test]
fn call_events_require_their_canonical_record_and_cardinality() {
    let sink = sink();
    assert!(
        errors(replace_event(&sink, "mark", None), canonical_records())
            .contains("missing event `mark`")
    );
    for (replacement, expected) in [
        (
            event("mark", [field("data", record_type("quent::nvtx::Mark"))]),
            "must be repeatable",
        ),
        (
            EventBuilder::new(ident("mark"), Cardinality::Multi)
                .with_field(field("data", record_type("quent::nvtx::RangeEnd")))
                .build()
                .unwrap(),
            "must contain exactly",
        ),
    ] {
        assert!(
            errors(
                replace_event(&sink, "mark", Some(replacement)),
                canonical_records()
            )
            .contains(expected)
        );
    }
    let extra = EntityBuilder::new(nvtx_path())
        .with_annotations(sink.annotations().clone())
        .with_events(sink.events().cloned())
        .with_event(event("extra", []))
        .build()
        .unwrap();
    assert!(errors(extra, canonical_records()).contains("unexpected event"));
}

#[test]
fn canonical_records_require_the_constraint_and_exact_fields() {
    let records = canonical_records();
    let record = records
        .iter()
        .find(|record| record.path() == &path("quent::nvtx::NvtxColor"))
        .unwrap();
    for (replacement, expected) in [
        (
            RecordBuilder::new(record.path().clone())
                .with_fields(record.fields().cloned())
                .build()
                .unwrap(),
            "must carry constraint",
        ),
        (
            RecordBuilder::new(record.path().clone())
                .with_annotations(record.annotations().clone())
                .with_fields(record.fields().cloned())
                .with_field(field("extra", DataType::Bool))
                .build()
                .unwrap(),
            "unexpected field",
        ),
    ] {
        let records = records
            .iter()
            .map(|record| {
                if record.path() == replacement.path() {
                    replacement.clone()
                } else {
                    record.clone()
                }
            })
            .collect();
        assert!(errors(sink(), records).contains(expected));
    }
}

#[test]
fn initialization_requires_a_once_only_ordinary_reference_without_data() {
    let sink = sink();
    let original = sink.event(&ident("initialized")).unwrap();
    assert!(
        errors(
            replace_event(&sink, "initialized", None),
            canonical_records()
        )
        .contains("missing event `initialized`")
    );
    let multi = EventBuilder::new(ident("initialized"), Cardinality::Multi)
        .with_fields(original.fields().cloned())
        .build()
        .unwrap();
    assert!(
        errors(
            replace_event(&sink, "initialized", Some(multi)),
            canonical_records()
        )
        .contains("cardinality `Once`")
    );
    for ty in [
        DataType::Option(Box::new(
            original.field(&ident("process")).unwrap().ty().clone(),
        )),
        DataType::EntityRef {
            data: Some(Box::new(DataType::U64)),
            annotations: Default::default(),
        },
        DataType::EntityRef {
            data: None,
            annotations: AnnotationsBuilder::new()
                .with_constraint(RefTreeConstraint::NAME, None)
                .build()
                .unwrap(),
        },
    ] {
        let replacement = event("initialized", [field("process", ty)]);
        assert!(
            !errors(
                replace_event(&sink, "initialized", Some(replacement)),
                canonical_records()
            )
            .is_empty()
        );
    }
    let extra = EventBuilder::new(ident("initialized"), Cardinality::Once)
        .with_fields(original.fields().cloned())
        .with_field(field("extra", DataType::Bool))
        .build()
        .unwrap();
    assert!(
        errors(
            replace_event(&sink, "initialized", Some(extra)),
            canonical_records()
        )
        .contains("only `process`")
    );
}

#[test]
fn annotations_reject_payloads_and_misplaced_constraints() {
    let annotations = AnnotationsBuilder::new()
        .with_constraint(NvtxConstraint::NAME, Some("unexpected".into()))
        .build()
        .unwrap();
    let annotated_sink = EntityBuilder::new(nvtx_path())
        .with_annotations(annotations.clone())
        .with_events(sink().events().cloned())
        .build()
        .unwrap();
    assert!(errors(annotated_sink, canonical_records()).contains("must not carry data"));
    let mut records = canonical_records();
    records.push(
        RecordBuilder::new(path("Other"))
            .with_annotations(annotations.clone())
            .build()
            .unwrap(),
    );
    assert!(
        errors(sink(), records)
            .contains("valid only on the canonical NVTX entity and canonical NVTX records")
    );
    let annotated_event = EventBuilder::new(ident("mark"), Cardinality::Multi)
        .with_annotations(annotations)
        .with_field(field("data", record_type("quent::nvtx::Mark")))
        .build()
        .unwrap();
    assert!(
        errors(
            replace_event(&sink(), "mark", Some(annotated_event)),
            canonical_records()
        )
        .contains("valid only")
    );
}

#[test]
fn annotated_entities_must_use_the_canonical_nvtx_path() {
    let canonical = sink();
    let renamed = EntityBuilder::try_new("CustomSink")
        .unwrap()
        .with_annotations(canonical.annotations().clone())
        .with_events(canonical.events().cloned())
        .build()
        .unwrap();
    assert!(errors(renamed, canonical_records()).contains("must be named `quent::nvtx::Sink`"));
}

#[test]
fn canonical_entity_requires_its_constraint() {
    let unannotated = EntityBuilder::new(nvtx_path())
        .with_events(sink().events().cloned())
        .build()
        .unwrap();
    assert!(errors(unannotated, canonical_records()).contains("must carry constraint"));
}
