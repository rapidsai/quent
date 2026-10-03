// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Decode owned NVTX records without accessing application memory.

use nvtx_events::{
    NvtxColor, NvtxEvent, NvtxEventAttributes, NvtxMessage, NvtxPayload, NvtxPayloadValue,
};
use nvtx_injection::record::{RawAttributes, RawEvent, RawMessage, RawString};
use nvtx_sys::ffi::nvtxPayloadType_t;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::NvtxEventEntity;

impl From<RawEvent> for NvtxEventEntity {
    fn from(record: RawEvent) -> Self {
        Self(convert(record))
    }
}

fn convert(record: RawEvent) -> NvtxEvent {
    match record {
        RawEvent::RangePush {
            domain,
            thread_id,
            attributes: raw,
        } => NvtxEvent::RangePush {
            domain,
            thread_id,
            attributes: attributes(raw),
        },
        RawEvent::RangePop { domain, thread_id } => NvtxEvent::RangePop { domain, thread_id },
        RawEvent::RangeStart {
            domain,
            range_id,
            attributes: raw,
        } => NvtxEvent::RangeStart {
            domain,
            range_id,
            attributes: attributes(raw),
        },
        RawEvent::RangeEnd { domain, range_id } => NvtxEvent::RangeEnd { domain, range_id },
        RawEvent::Mark {
            domain,
            attributes: raw,
        } => NvtxEvent::Mark {
            domain,
            attributes: attributes(raw),
        },
        RawEvent::DomainCreate { domain, name } => NvtxEvent::DomainCreate {
            domain,
            name: string(name),
        },
        RawEvent::DomainDestroy { domain } => NvtxEvent::DomainDestroy { domain },
        RawEvent::RegisterString {
            domain,
            handle,
            string: raw,
        } => NvtxEvent::RegisterString {
            domain,
            handle,
            string: string(raw),
        },
        RawEvent::NameCategory {
            domain,
            category,
            name,
        } => NvtxEvent::NameCategory {
            domain,
            category,
            name: string(name),
        },
        RawEvent::NameThread { thread_id, name } => NvtxEvent::NameThread {
            thread_id,
            name: string(name),
        },
        RawEvent::ResourceCreate {
            domain,
            handle,
            identifier_type,
            identifier,
            message: raw,
        } => NvtxEvent::ResourceCreate {
            domain,
            handle,
            identifier_type,
            identifier,
            message: raw.map(message),
        },
        RawEvent::ResourceDestroy { handle } => NvtxEvent::ResourceDestroy { handle },
    }
}

fn attributes(raw: RawAttributes) -> NvtxEventAttributes {
    NvtxEventAttributes {
        category: raw.category,
        color: raw.color.map(|raw| NvtxColor {
            color_type: raw.color_type,
            value: raw.value,
        }),
        message: raw.message.map(message),
        payload: raw.payload.map(|raw| {
            let bits = raw.bits;
            let value = match raw.payload_type {
                tag if tag == nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT64 as i32 => {
                    NvtxPayloadValue::Int64(bits as i64)
                }
                tag if tag == nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_DOUBLE as i32 => {
                    NvtxPayloadValue::Double(f64::from_bits(bits))
                }
                tag if tag == nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_UNSIGNED_INT32 as i32 => {
                    NvtxPayloadValue::UnsignedInt32(bits as u32)
                }
                tag if tag == nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_INT32 as i32 => {
                    NvtxPayloadValue::Int32(bits as i32)
                }
                tag if tag == nvtxPayloadType_t::NVTX_PAYLOAD_TYPE_FLOAT as i32 => {
                    NvtxPayloadValue::Float(f32::from_bits(bits as u32))
                }
                _ => NvtxPayloadValue::UnsignedInt64(bits),
            };
            NvtxPayload {
                payload_type: raw.payload_type,
                value,
            }
        }),
    }
}

fn message(raw: RawMessage) -> NvtxMessage {
    match raw {
        RawMessage::String(raw) => NvtxMessage::String(string(raw)),
        RawMessage::RegisteredHandle(handle) => NvtxMessage::RegisteredHandle(handle),
    }
}

fn string(raw: RawString) -> String {
    match raw {
        RawString::Bytes(bytes) => match String::from_utf8(bytes) {
            Ok(string) => string,
            Err(error) => {
                static WARNED: AtomicBool = AtomicBool::new(false);
                if !WARNED.swap(true, Ordering::Relaxed) {
                    eprintln!(
                        "nvtx-bridge: invalid UTF-8 in an NVTX string was replaced with U+FFFD. This warning fires once."
                    );
                }
                String::from_utf8_lossy(error.as_bytes()).into_owned()
            }
        },
        RawString::Wide(units) => units
            .into_iter()
            .map(|unit| char::from_u32(unit).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect(),
    }
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod tests;
