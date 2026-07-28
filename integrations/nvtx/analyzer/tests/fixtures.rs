// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Synthetic event-stream fixtures for the reconstruction tests.
//!
//! Hand-built streams, not captured ones: the malformed cases the core must
//! tolerate (out-of-order arrivals, duplicate timestamps, unclosed ranges) are
//! only reachable when the test controls every timestamp exactly.
//!
//! Included as a module by the reconstruction tests; also compiled standalone as
//! its own (test-free) target, hence the allow.
#![allow(dead_code)]

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};
use quent_events::Event;
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

/// The single entity id every fixture event is stamped with.
///
/// One capture session is one entity stream, so a fixed id keeps fixtures
/// deterministic and comparable across repeated builds.
pub fn stream_id() -> Uuid {
    Uuid::nil()
}

/// Wrap a raw [`NvtxEvent`] in the entity envelope at an exact timestamp.
pub fn at(timestamp: TimeUnixNanoSec, event: NvtxEvent) -> Event<NvtxEventEntity> {
    Event::new(stream_id(), timestamp, NvtxEventEntity(event))
}

/// Attributes carrying only an immediate string message.
pub fn message(text: &str) -> NvtxEventAttributes {
    NvtxEventAttributes {
        message: Some(NvtxMessage::String(text.to_owned())),
        ..Default::default()
    }
}

/// A `nvtxDomainRangeStartEx` opening the process-wide range `range_id`.
pub fn range_start(
    timestamp: TimeUnixNanoSec,
    domain: u64,
    range_id: u64,
    text: &str,
) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::RangeStart {
            domain,
            range_id,
            attributes: message(text),
        },
    )
}

/// A `nvtxDomainRangeEnd` closing the process-wide range `range_id`.
pub fn range_end(timestamp: TimeUnixNanoSec, domain: u64, range_id: u64) -> Event<NvtxEventEntity> {
    at(timestamp, NvtxEvent::RangeEnd { domain, range_id })
}

/// A `nvtxDomainRangePushEx` opening a nested range on `thread_id`.
///
/// The `(thread_id, domain)` pair is the nesting key, so both are explicit here:
/// a fixture that models two threads must vary `thread_id` for the interleaving
/// to mean anything.
pub fn range_push(
    timestamp: TimeUnixNanoSec,
    domain: u64,
    thread_id: u32,
    text: &str,
) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::RangePush {
            domain,
            thread_id,
            attributes: message(text),
        },
    )
}

/// A `nvtxDomainRangePop` closing the innermost push on `thread_id`.
pub fn range_pop(
    timestamp: TimeUnixNanoSec,
    domain: u64,
    thread_id: u32,
) -> Event<NvtxEventEntity> {
    at(timestamp, NvtxEvent::RangePop { domain, thread_id })
}

/// A `nvtxDomainRangePushEx` carrying a non-zero category.
///
/// Category is part of the statistics grouping key, so proving that grouping
/// needs a fixture that can vary it independently of the name and domain.
pub fn range_push_in_category(
    timestamp: TimeUnixNanoSec,
    domain: u64,
    thread_id: u32,
    category: u32,
    text: &str,
) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::RangePush {
            domain,
            thread_id,
            attributes: NvtxEventAttributes {
                category,
                ..message(text)
            },
        },
    )
}

/// A `nvtxDomainResourceCreate` opening the lifespan of resource `handle`.
///
/// `identifier_type` is the raw `nvtxResourceAttributes_t::identifierType` tag,
/// left explicit so a fixture can exercise both a core value and an
/// unrecognized one.
pub fn resource_create(
    timestamp: TimeUnixNanoSec,
    domain: u64,
    handle: u64,
    identifier_type: i32,
    identifier: u64,
    text: &str,
) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::ResourceCreate {
            domain,
            handle,
            identifier_type,
            identifier,
            message: Some(NvtxMessage::String(text.to_owned())),
        },
    )
}

/// A `nvtxDomainResourceDestroy` closing the lifespan of resource `handle`.
///
/// Deliberately takes no domain, because the NVTX event carries none — which is
/// exactly why the analyzer must match on the handle alone.
pub fn resource_destroy(timestamp: TimeUnixNanoSec, handle: u64) -> Event<NvtxEventEntity> {
    at(timestamp, NvtxEvent::ResourceDestroy { handle })
}

/// A `nvtxDomainMarkEx` instant — a non-range event, used to prove the replay
/// tolerates events it does not (yet) reconstruct.
pub fn mark(timestamp: TimeUnixNanoSec, domain: u64, text: &str) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::Mark {
            domain,
            attributes: message(text),
        },
    )
}
