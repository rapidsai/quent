// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! OS tests: canonical process and thread records are included when referenced.

use quent_os::{process_path, thread_path};
use quent_schema::test_utils::{ident, path};
use quent_schema::{DataType, Schema};
use quent_yaml::parse_from_str;

fn schema_of(src: &str) -> Schema {
    parse_from_str(src, None).expect("parses").schema
}

#[test]
fn process_record_is_added_when_referenced() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  MyProcess:
    events:
      init:
        attributes:
          process: quent::os::Process
",
    );

    let field = schema
        .entity(&path("MyProcess"))
        .unwrap()
        .event(&ident("init"))
        .unwrap()
        .field(&ident("process"))
        .unwrap();
    assert_eq!(field.ty(), &DataType::Record(process_path()));
    assert!(schema.record(&process_path()).is_some());
    assert!(schema.record(&thread_path()).is_none());
}

#[test]
fn thread_record_is_added_when_scoped_under_process() {
    let schema = schema_of(
        "\
quent: alpha
model: m
entities:
  MyProcess:
    events:
      init:
        attributes:
          process: quent::os::Process
  MyThread:
    events:
      init:
        attributes:
          thread: quent::os::Thread
          process: { scope-ref: MyProcess }
",
    );

    assert!(schema.record(&thread_path()).is_some());
    assert!(schema.record(&process_path()).is_some());
}
