// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_ref_target::RefTargetConstraint;
use quent_ref_tree::RefTreeConstraint;
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Field, Identifier, Path, Record,
    builder::{AnnotationsBuilder, EntityBuilder, EventBuilder, RecordBuilder},
};

use crate::NvtxConstraint;

/// Name of the once-only event that binds an NVTX stream to its process.
pub const INITIALIZED_EVENT: &str = "initialized";

/// `Message.kind` tag for an immediate string.
pub const MESSAGE_KIND_STRING: u8 = 0;

/// `Message.kind` tag for a registered-string handle.
pub const MESSAGE_KIND_REGISTERED_HANDLE: u8 = 1;

/// `Payload.value_kind` tag for an unsigned 64-bit integer.
pub const PAYLOAD_VALUE_KIND_UNSIGNED_INT64: u8 = 0;

/// `Payload.value_kind` tag for a signed 64-bit integer.
pub const PAYLOAD_VALUE_KIND_INT64: u8 = 1;

/// `Payload.value_kind` tag for an IEEE-754 double's raw bits.
pub const PAYLOAD_VALUE_KIND_DOUBLE: u8 = 2;

/// `Payload.value_kind` tag for an unsigned 32-bit integer.
pub const PAYLOAD_VALUE_KIND_UNSIGNED_INT32: u8 = 3;

/// `Payload.value_kind` tag for a signed 32-bit integer.
pub const PAYLOAD_VALUE_KIND_INT32: u8 = 4;

/// `Payload.value_kind` tag for an IEEE-754 float's raw bits.
pub const PAYLOAD_VALUE_KIND_FLOAT: u8 = 5;

/// `Payload.value_kind` tag for a pointer-sized handle's raw bits.
pub const PAYLOAD_VALUE_KIND_POINTER: u8 = 6;

/// Names of the captured NVTX events, in canonical declaration order.
pub const CAPTURE_EVENT_NAMES: [&str; 12] = [
    "range_push",
    "range_pop",
    "range_start",
    "range_end",
    "mark",
    "domain_create",
    "domain_destroy",
    "register_string",
    "name_category",
    "name_thread",
    "resource_create",
    "resource_destroy",
];

fn ident(name: &str) -> Identifier {
    Identifier::try_new(name).expect("canonical NVTX identifier is valid")
}

fn field(name: &str, ty: DataType) -> Field {
    Field::new(ident(name), ty, Annotations::default())
}

fn nvtx_annotations() -> Annotations {
    AnnotationsBuilder::new()
        .with_constraint(NvtxConstraint::NAME, None)
        .build()
        .expect("canonical NVTX annotations are valid")
}

fn event(
    name: &str,
    cardinality: Cardinality,
    fields: impl IntoIterator<Item = Field>,
) -> quent_schema::Event {
    EventBuilder::new(ident(name), cardinality)
        .with_fields(fields)
        .build()
        .expect("canonical NVTX event is valid")
}

/// Return the canonical path of the NVTX event-stream entity.
pub fn nvtx_event_path() -> Path {
    Path::try_new(["NvtxEvent"]).expect("canonical NVTX entity path is valid")
}

/// Return the canonical path of the NVTX color record.
pub fn color_path() -> Path {
    Path::try_new(["quent", "nvtx", "Color"]).expect("canonical NVTX record path is valid")
}

/// Return the canonical path of the NVTX message record.
pub fn message_path() -> Path {
    Path::try_new(["quent", "nvtx", "Message"]).expect("canonical NVTX record path is valid")
}

/// Return the canonical path of the NVTX payload record.
pub fn payload_path() -> Path {
    Path::try_new(["quent", "nvtx", "Payload"]).expect("canonical NVTX record path is valid")
}

/// Return the canonical path of the NVTX event-attributes record.
pub fn attributes_path() -> Path {
    Path::try_new(["quent", "nvtx", "Attributes"]).expect("canonical NVTX record path is valid")
}

/// Build the lossless canonical NVTX color record.
///
/// `color_type` retains the native tag even when it is unknown. `value` holds
/// the raw color bits.
pub fn color_record() -> Record {
    RecordBuilder::new(color_path())
        .with_annotations(nvtx_annotations())
        .with_fields([
            field("color_type", DataType::I32),
            field("value", DataType::U32),
        ])
        .build()
        .expect("canonical NVTX color record is valid")
}

/// Build the canonical tagged NVTX message record.
///
/// `kind` distinguishes immediate strings from registered handles. The
/// inactive optional field remains absent on the wire.
pub fn message_record() -> Record {
    RecordBuilder::new(message_path())
        .with_annotations(nvtx_annotations())
        .with_fields([
            field("kind", DataType::U8),
            field("string", DataType::Option(Box::new(DataType::String))),
            field(
                "registered_handle",
                DataType::Option(Box::new(DataType::U64)),
            ),
        ])
        .build()
        .expect("canonical NVTX message record is valid")
}

/// Build the lossless canonical NVTX payload record.
///
/// The native `payload_type` tag and an integration-owned `value_kind` tag
/// accompany the raw 64-bit representation. This preserves signed integers,
/// floating-point NaNs, and negative zero without schema-level unions.
pub fn payload_record() -> Record {
    RecordBuilder::new(payload_path())
        .with_annotations(nvtx_annotations())
        .with_fields([
            field("payload_type", DataType::I32),
            field("value_kind", DataType::U8),
            field("value_bits", DataType::U64),
        ])
        .build()
        .expect("canonical NVTX payload record is valid")
}

/// Build the canonical NVTX event-attributes record.
pub fn attributes_record() -> Record {
    RecordBuilder::new(attributes_path())
        .with_annotations(nvtx_annotations())
        .with_fields([
            field("category", DataType::U32),
            field(
                "color",
                DataType::Option(Box::new(DataType::Record(color_path()))),
            ),
            field(
                "message",
                DataType::Option(Box::new(DataType::Record(message_path()))),
            ),
            field(
                "payload",
                DataType::Option(Box::new(DataType::Record(payload_path()))),
            ),
        ])
        .build()
        .expect("canonical NVTX attributes record is valid")
}

/// Build the canonical NVTX stream entity bound to `process_target`.
///
/// The entity instance identifies one NVTX stream. Its `initialized` event
/// persists a typed, tree-forming scope reference to the owning OS-process
/// entity instance. The remaining events mirror the twelve captured NVTX call
/// kinds.
pub fn nvtx_event_entity(process_target: &Path) -> Entity {
    let process_ref = DataType::EntityRef {
        data: None,
        annotations: AnnotationsBuilder::new()
            .with_constraint(RefTargetConstraint::NAME, Some(process_target.to_string()))
            .with_constraint(RefTreeConstraint::NAME, None)
            .build()
            .expect("canonical NVTX process reference is valid"),
    };
    let attributes = || DataType::Record(attributes_path());

    EntityBuilder::new(nvtx_event_path())
        .with_annotations(nvtx_annotations())
        .with_events([
            event(
                INITIALIZED_EVENT,
                Cardinality::Once,
                [field("process", process_ref)],
            ),
            event(
                "range_push",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("thread_id", DataType::U32),
                    field("attributes", attributes()),
                ],
            ),
            event(
                "range_pop",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("thread_id", DataType::U32),
                ],
            ),
            event(
                "range_start",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("range_id", DataType::U64),
                    field("attributes", attributes()),
                ],
            ),
            event(
                "range_end",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("range_id", DataType::U64),
                ],
            ),
            event(
                "mark",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("attributes", attributes()),
                ],
            ),
            event(
                "domain_create",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("name", DataType::String),
                ],
            ),
            event(
                "domain_destroy",
                Cardinality::Multi,
                [field("domain", DataType::U64)],
            ),
            event(
                "register_string",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("handle", DataType::U64),
                    field("string", DataType::String),
                ],
            ),
            event(
                "name_category",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("category", DataType::U32),
                    field("name", DataType::String),
                ],
            ),
            event(
                "name_thread",
                Cardinality::Multi,
                [
                    field("thread_id", DataType::U32),
                    field("name", DataType::String),
                ],
            ),
            event(
                "resource_create",
                Cardinality::Multi,
                [
                    field("domain", DataType::U64),
                    field("handle", DataType::U64),
                    field("identifier_type", DataType::I32),
                    field("identifier", DataType::U64),
                    field(
                        "message",
                        DataType::Option(Box::new(DataType::Record(message_path()))),
                    ),
                ],
            ),
            event(
                "resource_destroy",
                Cardinality::Multi,
                [field("handle", DataType::U64)],
            ),
        ])
        .build()
        .expect("canonical NVTX entity is valid")
}
