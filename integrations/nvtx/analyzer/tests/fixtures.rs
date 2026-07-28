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
