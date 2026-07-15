// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX payload vocabulary.
//!
//! Two tiers live here:
//!
//! * The payload **union** carried on core `nvtxEventAttributes` — captured
//!   verbatim (undecoded) ([`NvtxPayload`]).
//! * The payload-**extension** vocabulary (schema/enum registration, binary
//!   blobs) — **defined but not wired** ([`PayloadExtensionEvent`]). These are
//!   deferred to a later phase (alongside payload decoding); they exist now only
//!   so the stream can carry them later without a vocabulary-breaking change. No
//!   capture path emits them today.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The payload union carried on core `nvtxEventAttributes`, captured verbatim
/// (undecoded).
///
/// Carries the raw `NVTX_PAYLOAD_TYPE_*` tag alongside the scalar value the
/// union holds. Interpretation/decoding is deferred to the analyzer.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct NvtxPayload {
    /// Raw `NVTX_PAYLOAD_TYPE_*` tag, preserved verbatim.
    pub payload_type: i32,
    /// The scalar value carried by the union.
    pub value: NvtxPayloadValue,
}

/// The scalar members of the core payload union.
///
/// Each variant mirrors one member of NVTX's payload union; values are captured
/// as-is with no reinterpretation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum NvtxPayloadValue {
    /// `ullValue` — unsigned 64-bit integer.
    UnsignedInt64(u64),
    /// `llValue` — signed 64-bit integer.
    Int64(i64),
    /// `dValue` — double-precision float.
    Double(f64),
    /// `uiValue` — unsigned 32-bit integer.
    UnsignedInt32(u32),
    /// `iValue` — signed 32-bit integer.
    Int32(i32),
    /// `fValue` — single-precision float.
    Float(f32),
    /// A pointer-sized handle, captured as raw bits.
    ///
    /// Reserved for a future payload-extension mapping; **not** emitted by core
    /// capture (unknown core payload tags fall back to [`Self::UnsignedInt64`]).
    Pointer(u64),
}

/// NVTX payload-**extension** vocabulary.
///
/// Deferred: these variants are defined so the event stream can carry
/// payload-extension data in a later phase without a vocabulary-breaking change,
/// but they are **not** wired into [`NvtxEvent`](crate::NvtxEvent) and no capture
/// path emits them yet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum PayloadExtensionEvent {
    /// Register a payload schema (`nvtxPayloadSchemaRegister`).
    SchemaRegister {
        /// Raw domain handle the schema is registered against.
        domain: u64,
        /// Raw schema id returned by NVTX.
        schema_id: u64,
        /// Raw schema descriptor bytes, captured verbatim.
        descriptor: Vec<u8>,
    },
    /// Register a payload enum (`nvtxPayloadEnumRegister`).
    EnumRegister {
        /// Raw domain handle the enum is registered against.
        domain: u64,
        /// Raw enum id returned by NVTX.
        enum_id: u64,
        /// Raw enum descriptor bytes, captured verbatim.
        descriptor: Vec<u8>,
    },
    /// A binary payload blob attached to an event (`nvtxPayloadData_t`).
    BinaryPayload {
        /// Raw schema id the blob conforms to.
        schema_id: u64,
        /// Raw payload bytes, captured verbatim (decoded in a later phase).
        bytes: Vec<u8>,
    },
}
