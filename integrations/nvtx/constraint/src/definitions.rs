// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_schema::{
    Annotations, DataType, Field, Identifier, Path, Record,
    builder::{AnnotationsBuilder, RecordBuilder},
};

use crate::NvtxConstraint;

pub(crate) const EVENTS: [(&str, &str); 15] = [
    ("range_push", "RangePush"),
    ("range_pop", "RangePop"),
    ("range_start", "RangeStart"),
    ("range_end", "RangeEnd"),
    ("mark", "Mark"),
    ("domain_create", "DomainCreate"),
    ("domain_destroy", "DomainDestroy"),
    ("register_string", "RegisterString"),
    ("name_category", "NameCategory"),
    ("name_thread", "NameThread"),
    ("resource_create", "ResourceCreate"),
    ("resource_destroy", "ResourceDestroy"),
    ("schema_register", "SchemaRegister"),
    ("enum_register", "EnumRegister"),
    ("binary_payload", "BinaryPayload"),
];

pub(crate) fn ident(name: &str) -> Identifier {
    Identifier::try_new(name).expect("canonical NVTX identifier is valid")
}

pub(crate) fn path(name: &str) -> Path {
    Path::try_new(["quent", "nvtx", name]).expect("canonical NVTX path is valid")
}

pub(crate) fn field(name: &str, ty: DataType) -> Field {
    Field::new(ident(name), ty, Annotations::default())
}

fn record(name: &str, fields: impl IntoIterator<Item = (&'static str, DataType)>) -> Record {
    RecordBuilder::new(path(name))
        .with_annotations(
            AnnotationsBuilder::new()
                .with_constraint(NvtxConstraint::NAME, None)
                .build()
                .expect("canonical NVTX annotations are valid"),
        )
        .with_fields(fields.into_iter().map(|(name, ty)| field(name, ty)))
        .build()
        .expect("canonical NVTX record is valid")
}

/// Return the complete canonical NVTX record vocabulary under `quent::nvtx`.
///
/// `NvtxMessage` has optional `string` and `registered_handle` fields; producers
/// must set exactly one. `NvtxPayload` carries a raw `payload_type: I32` and
/// `value: U64`: signed integers use their two's-complement bits and floating-point
/// values use their IEEE 754 bits. For 32-bit members, the upper 32 bits are zero.
/// Handles remain unresolved integers; domain zero denotes the default domain.
pub fn canonical_records() -> Vec<Record> {
    use DataType::{I32, String, U32, U64};
    let reference = |name| DataType::Record(path(name));
    let optional = |ty| DataType::Option(Box::new(ty));
    let bytes = || DataType::List(Box::new(DataType::U8));
    vec![
        record(
            "NvtxMessage",
            [
                ("string", optional(String)),
                ("registered_handle", optional(U64)),
            ],
        ),
        record("NvtxColor", [("color_type", I32), ("value", U32)]),
        record("NvtxPayload", [("payload_type", I32), ("value", U64)]),
        record(
            "NvtxEventAttributes",
            [
                ("category", U32),
                ("color", optional(reference("NvtxColor"))),
                ("message", optional(reference("NvtxMessage"))),
                ("payload", optional(reference("NvtxPayload"))),
            ],
        ),
        record(
            "RangePush",
            [
                ("domain", U64),
                ("thread_id", U32),
                ("attributes", reference("NvtxEventAttributes")),
            ],
        ),
        record("RangePop", [("domain", U64), ("thread_id", U32)]),
        record(
            "RangeStart",
            [
                ("domain", U64),
                ("range_id", U64),
                ("attributes", reference("NvtxEventAttributes")),
            ],
        ),
        record("RangeEnd", [("domain", U64), ("range_id", U64)]),
        record(
            "Mark",
            [
                ("domain", U64),
                ("attributes", reference("NvtxEventAttributes")),
            ],
        ),
        record("DomainCreate", [("domain", U64), ("name", String)]),
        record("DomainDestroy", [("domain", U64)]),
        record(
            "RegisterString",
            [("domain", U64), ("handle", U64), ("string", String)],
        ),
        record(
            "NameCategory",
            [("domain", U64), ("category", U32), ("name", String)],
        ),
        record("NameThread", [("thread_id", U32), ("name", String)]),
        record(
            "ResourceCreate",
            [
                ("domain", U64),
                ("handle", U64),
                ("identifier_type", I32),
                ("identifier", U64),
                ("message", optional(reference("NvtxMessage"))),
            ],
        ),
        record("ResourceDestroy", [("handle", U64)]),
        record(
            "SchemaRegister",
            [("domain", U64), ("schema_id", U64), ("descriptor", bytes())],
        ),
        record(
            "EnumRegister",
            [("domain", U64), ("enum_id", U64), ("descriptor", bytes())],
        ),
        record("BinaryPayload", [("schema_id", U64), ("bytes", bytes())]),
    ]
}
