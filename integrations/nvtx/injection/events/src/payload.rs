// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX payload extension types.
//!
//! These represent raw payload schemas, enums, and binary payload data
//! from the NVTX payload extension (`nvToolsExtPayload.h`). The injection
//! forwards them as opaque data; the analyzer interprets them.

use serde::{Deserialize, Serialize};

/// A payload schema registration event.
#[derive(Debug, Deserialize, Serialize)]
pub struct SchemaRegister {
    pub domain_handle_id: Option<u64>,
    pub schema_id: u64,
    pub schema: RawPayloadSchema,
}

/// A payload enum registration event.
#[derive(Debug, Deserialize, Serialize)]
pub struct EnumRegister {
    pub domain_handle_id: Option<u64>,
    pub schema_id: u64,
    pub entries: Vec<RawPayloadEnum>,
}

/// A push event with payload extension data.
#[derive(Debug, Deserialize, Serialize)]
pub struct PushPayload {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

/// A pop event with payload extension data.
#[derive(Debug, Deserialize, Serialize)]
pub struct PopPayload {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

/// A range-start event with payload extension data.
#[derive(Debug, Deserialize, Serialize)]
pub struct RangeStartPayload {
    pub range_handle_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

/// A range-end event with payload extension data.
#[derive(Debug, Deserialize, Serialize)]
pub struct RangeEndPayload {
    pub range_handle_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

/// A mark event with payload extension data.
#[derive(Debug, Deserialize, Serialize)]
pub struct MarkPayload {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub payloads: Vec<RawPayloadData>,
}

/// Raw binary payload data, identified by schema ID.
///
/// The injection captures the raw bytes without interpretation. The
/// analyzer uses the schema registry (from prior `SchemaRegister` events)
/// to decode the bytes.
#[derive(Debug, Deserialize, Serialize)]
pub struct RawPayloadData {
    pub schema_id: u64,
    pub data: Vec<u8>,
}

/// A raw payload schema definition, captured verbatim from
/// `nvtxPayloadSchemaRegister`.
///
/// Stored as raw bytes at the event layer. The analyzer (Phase 4) will
/// define structured schema parsing.
#[derive(Debug, Deserialize, Serialize)]
pub struct RawPayloadSchema {
    pub data: Vec<u8>,
}

/// A raw payload enum entry, captured from `nvtxPayloadEnumRegister`.
#[derive(Debug, Deserialize, Serialize)]
pub struct RawPayloadEnum {
    pub name: String,
    pub value: u64,
}
