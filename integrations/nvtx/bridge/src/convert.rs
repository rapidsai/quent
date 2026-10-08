// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Decode owned NVTX records without accessing application memory.

use nvtx_injection::record::{Attributes, Message, Record, String as RecordString};
use nvtx_sys::ffi::nvtxPayloadType_t;
use quent_nvtx_events::{
    NvtxColor, NvtxEvent, NvtxEventAttributes, NvtxMessage, NvtxPayload, NvtxPayloadValue,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) fn current_thread_id() -> u32 {
    thread_local! {
        static CACHED_TID: u32 = compute_thread_id();
    }
    CACHED_TID
        .try_with(|thread_id| *thread_id)
        .unwrap_or_else(|_| compute_thread_id())
}

fn compute_thread_id() -> u32 {
    // The syscall works with older Linux libc versions that lack gettid().
    // SAFETY: SYS_gettid takes no arguments and returns the calling thread's ID.
    unsafe { libc::syscall(libc::SYS_gettid) as u32 }
}

/// Convert an owned injection record into an NVTX event.
///
/// Push/pop records receive the calling thread's OS ID.
pub fn convert(record: Record) -> NvtxEvent {
    convert_with_thread_id(record, None)
}

pub(crate) fn convert_with_thread_id(record: Record, caller_thread_id: Option<u32>) -> NvtxEvent {
    match record {
        Record::RangePush {
            domain,
            attributes: raw,
        } => NvtxEvent::RangePush {
            domain,
            thread_id: caller_thread_id.unwrap_or_else(current_thread_id),
            attributes: attributes(raw),
        },
        Record::RangePop { domain } => NvtxEvent::RangePop {
            domain,
            thread_id: caller_thread_id.unwrap_or_else(current_thread_id),
        },
        Record::RangeStart {
            domain,
            range_id,
            attributes: raw,
        } => NvtxEvent::RangeStart {
            domain,
            range_id,
            attributes: attributes(raw),
        },
        Record::RangeEnd { domain, range_id } => NvtxEvent::RangeEnd { domain, range_id },
        Record::Mark {
            domain,
            attributes: raw,
        } => NvtxEvent::Mark {
            domain,
            attributes: attributes(raw),
        },
        Record::DomainCreate { domain, name } => NvtxEvent::DomainCreate {
            domain,
            name: string(name),
        },
        Record::DomainDestroy { domain } => NvtxEvent::DomainDestroy { domain },
        Record::RegisterString {
            domain,
            handle,
            string: raw,
        } => NvtxEvent::RegisterString {
            domain,
            handle,
            string: string(raw),
        },
        Record::NameCategory {
            domain,
            category,
            name,
        } => NvtxEvent::NameCategory {
            domain,
            category,
            name: string(name),
        },
        Record::NameThread { thread_id, name } => NvtxEvent::NameThread {
            thread_id,
            name: string(name),
        },
        Record::ResourceCreate {
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
        Record::ResourceDestroy { handle } => NvtxEvent::ResourceDestroy { handle },
    }
}

fn attributes(raw: Attributes) -> NvtxEventAttributes {
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

fn message(raw: Message) -> NvtxMessage {
    match raw {
        Message::String(raw) => NvtxMessage::String(string(raw)),
        Message::RegisteredHandle(handle) => NvtxMessage::RegisteredHandle(handle),
    }
}

fn string(raw: RecordString) -> String {
    match raw {
        RecordString::Bytes(bytes) => match String::from_utf8(bytes) {
            Ok(string) => string,
            Err(error) => {
                static WARNED: AtomicBool = AtomicBool::new(false);
                if !WARNED.swap(true, Ordering::Relaxed) {
                    eprintln!(
                        "quent-nvtx-bridge: invalid UTF-8 in an NVTX string was replaced with U+FFFD. This warning fires once."
                    );
                }
                String::from_utf8_lossy(error.as_bytes()).into_owned()
            }
        },
        RecordString::Wide(units) => units
            .into_iter()
            .map(|unit| char::from_u32(unit).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect(),
    }
}
