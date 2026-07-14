// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX payload vocabulary.
//!
//! Two tiers live here:
//!
//! * The CORE `nvtxEventAttributes` payload **union** — captured verbatim
//!   (undecoded) in Phase 1 ([`NvtxPayload`]).
//! * The payload-**extension** vocabulary (schema/enum registration, binary
//!   blobs) — **defined but not wired** ([`PayloadExtensionEvent`]). Per D-12
//!   these are DEFERRED to a later phase (natural home alongside `PAY-01`
//!   decode); they exist now only so the stream can carry them later without a
//!   vocabulary-breaking change. No capture path emits them today.

use serde::{Deserialize, Serialize};

/// The CORE `nvtxEventAttributes` payload union, captured verbatim (undecoded).
///
/// Carries the raw `NVTX_PAYLOAD_TYPE_*` tag alongside the scalar value the
/// union holds. Interpretation/decoding is deferred to the analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct NvtxPayload {
    /// Raw `NVTX_PAYLOAD_TYPE_*` tag, preserved verbatim.
    pub payload_type: i32,
    /// The scalar value carried by the union.
    pub value: NvtxPayloadValue,
}

/// The scalar members of the CORE payload union.
///
/// Each variant mirrors one member of NVTX's payload union; values are captured
/// as-is with no reinterpretation.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
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
    /// IN-03: reserved for a future payload-extension mapping; **not** emitted
    /// by CORE capture. `read_payload` never produces this variant in Phase 1
    /// (unknown CORE tags fall back to [`Self::UnsignedInt64`]).
    Pointer(u64),
}

/// NVTX payload-**extension** vocabulary.
///
/// DEFERRED per D-12: these variants are defined so the event stream can carry
/// payload-extension data in a later phase without a vocabulary-breaking change,
/// but they are **not** wired into [`NvtxEvent`](crate::NvtxEvent) and no capture
/// path emits them yet. Their natural home is alongside `PAY-01` decode.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(payload: &NvtxPayload) -> NvtxPayload {
        let json = serde_json::to_string(payload).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn core_payload_union_round_trips_each_scalar_verbatim() {
        let cases = [
            NvtxPayload {
                payload_type: 1,
                value: NvtxPayloadValue::UnsignedInt64(u64::MAX),
            },
            NvtxPayload {
                payload_type: 2,
                value: NvtxPayloadValue::Int64(i64::MIN),
            },
            NvtxPayload {
                payload_type: 3,
                value: NvtxPayloadValue::Double(3.5),
            },
            NvtxPayload {
                payload_type: 4,
                value: NvtxPayloadValue::UnsignedInt32(42),
            },
            NvtxPayload {
                payload_type: 5,
                value: NvtxPayloadValue::Int32(-7),
            },
            NvtxPayload {
                payload_type: 6,
                value: NvtxPayloadValue::Float(0.25),
            },
            NvtxPayload {
                payload_type: 7,
                value: NvtxPayloadValue::Pointer(0xDEAD_BEEF),
            },
        ];

        for payload in &cases {
            assert_eq!(&round_trip(payload), payload);
        }
    }

    #[test]
    fn deferred_payload_extension_vocabulary_round_trips() {
        let event = PayloadExtensionEvent::BinaryPayload {
            schema_id: 9,
            bytes: vec![1, 2, 3, 4],
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let back: PayloadExtensionEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, event);
    }
}
