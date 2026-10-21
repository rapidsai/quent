// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Decode owned NVTX records without accessing application memory.

use nvtx_injection::record::{Attributes, Message, Record, String as RecordString};
use quent_nvtx_sink::quent::nvtx::*;
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
pub fn convert(record: Record) -> SinkEvent {
    convert_with_thread_id(record, None)
}

pub(crate) fn convert_with_thread_id(record: Record, caller_thread_id: Option<u32>) -> SinkEvent {
    match record {
        Record::RangePush {
            domain,
            attributes: raw,
        } => SinkEvent::RangePush {
            data: RangePush {
                domain,
                thread_id: caller_thread_id.unwrap_or_else(current_thread_id),
                attributes: attributes(raw),
            },
        },
        Record::RangePop { domain } => SinkEvent::RangePop {
            data: RangePop {
                domain,
                thread_id: caller_thread_id.unwrap_or_else(current_thread_id),
            },
        },
        Record::RangeStart {
            domain,
            range_id,
            attributes: raw,
        } => SinkEvent::RangeStart {
            data: RangeStart {
                domain,
                range_id,
                attributes: attributes(raw),
            },
        },
        Record::RangeEnd { domain, range_id } => SinkEvent::RangeEnd {
            data: RangeEnd { domain, range_id },
        },
        Record::Mark {
            domain,
            attributes: raw,
        } => SinkEvent::Mark {
            data: Mark {
                domain,
                attributes: attributes(raw),
            },
        },
        Record::DomainCreate { domain, name } => SinkEvent::DomainCreate {
            data: DomainCreate {
                domain,
                name: string(name),
            },
        },
        Record::DomainDestroy { domain } => SinkEvent::DomainDestroy {
            data: DomainDestroy { domain },
        },
        Record::RegisterString {
            domain,
            handle,
            string: raw,
        } => SinkEvent::RegisterString {
            data: RegisterString {
                domain,
                handle,
                string: string(raw),
            },
        },
        Record::NameCategory {
            domain,
            category,
            name,
        } => SinkEvent::NameCategory {
            data: NameCategory {
                domain,
                category,
                name: string(name),
            },
        },
        Record::NameThread { thread_id, name } => SinkEvent::NameThread {
            data: NameThread {
                thread_id,
                name: string(name),
            },
        },
        Record::ResourceCreate {
            domain,
            handle,
            identifier_type,
            identifier,
            message: raw,
        } => SinkEvent::ResourceCreate {
            data: ResourceCreate {
                domain,
                handle,
                identifier_type,
                identifier,
                message: raw.map(message),
            },
        },
        Record::ResourceDestroy { handle } => SinkEvent::ResourceDestroy {
            data: ResourceDestroy { handle },
        },
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
        payload: raw.payload.map(|raw| NvtxPayload {
            payload_type: raw.payload_type,
            value: raw.bits,
        }),
    }
}

fn message(raw: Message) -> NvtxMessage {
    match raw {
        Message::String(raw) => NvtxMessage {
            string: Some(string(raw)),
            registered_handle: None,
        },
        Message::RegisteredHandle(handle) => NvtxMessage {
            string: None,
            registered_handle: Some(handle),
        },
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
