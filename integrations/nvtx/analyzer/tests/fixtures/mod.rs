// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Synthetic event-stream fixtures for the reconstruction tests.
//!
//! Hand-built rather than captured: the malformed cases the core must tolerate
//! are only reachable when the test controls every timestamp exactly.
//!
//! A directory module rather than `tests/fixtures.rs` because Cargo registers
//! every `.rs` directly under `tests/` as its own integration-test binary, and
//! this file has no `#[test]`s. Each consumer compiles it in full but uses only
//! part of it, hence the allow.
#![allow(dead_code)]

use nvtx_analyzer::{NvtxModel, NvtxSpan, SpanId};
use nvtx_bridge::NvtxEventEntity;
use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};
use quent_events::Event;
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

/// `NVTX_RESOURCE_TYPE_GENERIC_POINTER`, composed by `NVTX_RESOURCE_MAKE_TYPE`.
pub const GENERIC_POINTER: i32 = 0x0001_0001;

/// The single span whose resolved name is `name`.
///
/// Asserts uniqueness, so a test meaning to pin one span cannot pass when
/// reconstruction produced two.
pub fn span<'a>(model: &'a NvtxModel, name: &str) -> &'a NvtxSpan {
    unique(model.spans().iter(), name)
}

/// The single *resource* span whose resolved name is `name`.
pub fn resource<'a>(model: &'a NvtxModel, name: &str) -> &'a NvtxSpan {
    unique(model.resources(), name)
}

/// The [`SpanId`] of the single span named `name` — the identity a `parent`
/// reference must equal.
pub fn span_id(model: &NvtxModel, name: &str) -> SpanId {
    let index = model
        .spans()
        .iter()
        .position(|span| span.name == name)
        .unwrap_or_else(|| panic!("no span named {name:?}"));
    SpanId(index)
}

/// The one span in `spans` named `name`, panicking if there is not exactly one.
fn unique<'a>(spans: impl Iterator<Item = &'a NvtxSpan>, name: &str) -> &'a NvtxSpan {
    let mut matches = spans.filter(|span| span.name == name);
    let found = matches
        .next()
        .unwrap_or_else(|| panic!("no span named {name:?}"));
    assert!(
        matches.next().is_none(),
        "more than one span named {name:?}"
    );
    found
}

/// The single entity id every fixture event is stamped with — one capture
/// session is one entity stream.
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
/// `(thread_id, domain)` is the nesting key, so both are explicit.
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

/// A `nvtxDomainRangePushEx` carrying a non-zero category, which the statistics
/// grouping key needs varied independently of name and domain.
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
/// `identifier_type` is explicit so a fixture can exercise both a core value
/// and an unrecognized one.
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
/// Takes no domain because the NVTX event carries none — which is why the
/// analyzer must match on the handle alone.
pub fn resource_destroy(timestamp: TimeUnixNanoSec, handle: u64) -> Event<NvtxEventEntity> {
    at(timestamp, NvtxEvent::ResourceDestroy { handle })
}

/// A `nvtxDomainMarkEx` instant, which also lets a fixture set the trace end
/// without opening a range.
pub fn mark(timestamp: TimeUnixNanoSec, domain: u64, text: &str) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::Mark {
            domain,
            attributes: message(text),
        },
    )
}
