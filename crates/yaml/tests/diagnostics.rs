// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Diagnostics tests: every rejection carries a useful message.

use quent_yaml::{Error, Origin, parse_from_str};

const HEADER: &str = "\
quent: alpha
model: m
";

/// Parse `src`, expect failure, and assert one diagnostic contains all
/// `needles` (checked against message and help).
#[track_caller]
fn expect_raw(src: &str, needles: &[&str]) {
    let diagnostics = match parse_from_str(src, None) {
        Err(Error::Invalid(d)) => d,
        Err(e) => panic!("expected diagnostics, got {e:?}"),
        Ok(_) => panic!("expected failure, parsed fine:\n{src}"),
    };
    let matched = diagnostics.iter().any(|d| {
        needles.iter().all(|needle| {
            d.message.contains(needle) || d.help.as_deref().is_some_and(|h| h.contains(needle))
        })
    });
    assert!(
        matched,
        "no diagnostic containing {needles:?}; got:\n{diagnostics}"
    );
}

/// Like [`expect_raw`], prefixing the standard `quent: alpha` / `model: m` header
/// so a test spells out only the body under scrutiny.
#[track_caller]
fn expect_error(body: &str, needles: &[&str]) {
    expect_raw(&format!("{HEADER}{body}"), needles);
}

#[test]
fn bad_format_version() {
    expect_raw(
        "\
quent: beta
model: m
",
        &["unsupported format version `beta`"],
    );
}

#[test]
fn event_cardinality_required() {
    expect_error(
        "\
entities:
  E:
    events:
      started:
        doc: x
",
        &["event must declare a cardinality"],
    );
    expect_error(
        "\
entities:
  E:
    events:
      started:
        once: {}
        multi: {}
",
        &["both `once` and `multi`"],
    );
}

#[test]
fn malformed_type_name() {
    // A compact Rust-style spelling is just an unusable bare type name.
    expect_error(
        "\
records:
  R:
    fields:
      f: Vec<u8>
",
        &["invalid type `Vec<u8>`"],
    );
}

#[test]
fn invalid_and_reserved_names() {
    expect_error(
        "\
records:
  'has space':
",
        &["invalid name `has space`"],
    );
    expect_error(
        "\
records:
  string:
    fields: { x: u8 }
",
        &["`string` is a reserved type name"],
    );
}

#[test]
fn unknown_record_reference() {
    expect_error(
        "\
records:
  R:
    fields:
      f: Ghost
",
        &["unresolved reference"],
    );
}

#[test]
fn recursive_record() {
    expect_error(
        "\
records:
  Node:
    fields:
      next: { option: Node }
",
        &["record `Node` is recursive"],
    );
}

#[test]
fn invalid_sibling_names_do_not_panic() {
    // Two records with invalid names must both surface as diagnostics rather
    // than reaching the builder as a shared placeholder and panicking.
    expect_error(
        "\
records:
  'a b':
  'c d':
",
        &["invalid name `a b`"],
    );
}

#[test]
fn empty_annotation_name() {
    expect_error(
        "\
constraints:
  '': x
",
        &["constraint name must not be empty"],
    );
}

#[test]
fn syntax_error_has_a_location() {
    let Err(Error::Invalid(diagnostics)) = parse_from_str(
        "\
quent: alpha
model: [
",
        None,
    ) else {
        panic!("expected failure");
    };
    assert!(
        diagnostics
            .iter()
            .any(|d| matches!(d.origin, Origin::Location { .. })),
        "parse errors should carry a location: {diagnostics}"
    );
}
