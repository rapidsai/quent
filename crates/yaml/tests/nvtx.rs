// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_nvtx::{NvtxConstraint, nvtx_path};
use quent_yaml::parse_from_str;

const PROCESS: &str = "\
quent: alpha
model: Application
integrations:
  nvtx: true
entities:
  Process:
    events:
      started:
        attributes:
          identity: { os: process }
";

#[test]
fn integration_adds_one_canonical_entity_and_its_records() {
    let parsed = parse_from_str(PROCESS, None).unwrap();
    assert!(parsed.warnings.is_empty());
    let schema = parsed.schema;
    assert_eq!(schema.entities().count(), 2);
    let entity = schema.entity(&nvtx_path()).unwrap();
    assert!(entity.annotations().has_constraint(NvtxConstraint::NAME));
    assert_eq!(
        entity
            .annotations()
            .metadata("quent.codegen.rust.type")
            .unwrap()
            .data(),
        Some("::quent_nvtx_sink::quent::nvtx::SinkEvent")
    );
    for record in quent_nvtx::canonical_records() {
        assert_eq!(
            schema
                .record(record.path())
                .unwrap()
                .annotations()
                .metadata("quent.codegen.rust.type")
                .unwrap()
                .data(),
            Some(format!("::quent_nvtx_sink::quent::nvtx::{}", record.path().name()).as_str())
        );
    }
}

#[test]
fn nvtx_requires_an_os_process() {
    let source = PROCESS.replace("identity: { os: process }", "identity: u32");
    let errors = parse_from_str(source, None).unwrap_err().to_string();
    assert!(
        errors.contains("at least one entity representing an OS process"),
        "{errors}"
    );
}

#[test]
fn multiple_process_types_share_the_canonical_nvtx_entity() {
    let source = "\
quent: alpha
model: Application
integrations:
  nvtx: true
entities:
  ProcessA:
    events:
      started:
        attributes:
          identity: { os: process }
  ProcessB:
    events:
      started:
        attributes:
          identity: { os: process }
";
    let parsed = parse_from_str(source, None).unwrap();
    assert!(parsed.warnings.is_empty());
    assert_eq!(parsed.schema.entities().count(), 3);
    assert!(parsed.schema.entity(&nvtx_path()).is_some());
}

#[test]
fn constraint_annotations_must_come_from_the_integration() {
    let source = PROCESS.replace(
        "  Process:\n",
        "  Process:\n    constraints:\n      quent.nvtx.v0.1.0:\n",
    );
    let errors = parse_from_str(source, None).unwrap_err().to_string();
    assert!(errors.contains("`integrations.nvtx: true`"), "{errors}");
}

#[test]
fn absent_and_disabled_integrations_add_no_nvtx_elements() {
    for source in [
        PROCESS.replace("integrations:\n  nvtx: true\n", ""),
        PROCESS.replace("nvtx: true", "nvtx: false"),
    ] {
        let schema = parse_from_str(source, None).unwrap().schema;
        assert_eq!(schema.entities().count(), 1);
        assert_eq!(schema.records().count(), 1);
    }
}
