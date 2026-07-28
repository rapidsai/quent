// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resource lifespan reconstruction — the third match key.
//!
//! `nvtxDomainResourceCreate` associates a handle with a name and an identifier;
//! `nvtxDomainResourceDestroy` releases it. The interval between them is a
//! lifespan, and this module reconstructs it as an
//! [`NvtxSpan`] with [`SpanKind::Resource`].
//!
//! **The match key is the handle alone.** Not `(domain, handle)` — and that is
//! not a simplification, it is forced by the vocabulary:
//! [`NvtxEvent::ResourceDestroy`](nvtx_events::NvtxEvent::ResourceDestroy)
//! carries *only* a handle, because the underlying NVTX call does. Keying on the
//! pair would not fail loudly on a real stream; every destroy would simply miss
//! its create, and every resource would silently reconstruct as a leaked
//! lifespan closed at trace end. The domain is therefore recovered from the
//! create and carried forward.
//!
//! A resource is *structurally* just a span. `nvtxDomainResourceCreate` says a
//! handle exists and what it is called — nothing about how large it is or how
//! much of it is in use — so this module models exactly that and infers no
//! further semantics (D-10). The only thing it adds beyond an interval and a
//! name is a label for the raw `identifierType` tag, and that label is a total
//! function with a raw pass-through for anything outside the core set.

use std::collections::HashMap;

use quent_time::TimeUnixNanoSec;
use tracing::warn;

use crate::span::{NvtxSpan, SpanKind};

/// The class of core/generic NVTX resource types (`NVTX_RESOURCE_CLASS_GENERIC`).
///
/// `nvToolsExt.h` composes an identifier type as
/// `NVTX_RESOURCE_MAKE_TYPE(CLASS, INDEX) = (CLASS << 16) | INDEX`, so the core
/// generic values are `0x0001_0001`..=`0x0001_0004`. Other classes (CUDA, CUDA
/// runtime, OpenCL, D3D, sync) are extension surfaces this core-only slice
/// deliberately does not interpret.
const CLASS_GENERIC: i32 = 1;

/// Compose the raw `identifierType` value for a core class and index.
const fn resource_type(class: i32, index: i32) -> i32 {
    (class << 16) | index
}

/// `NVTX_RESOURCE_TYPE_UNKNOWN`.
///
/// Also what the capture layer records when the attribute struct was null or too
/// short to contain the field, so it is a legitimate "not stated" rather than an
/// anomaly.
const TYPE_UNKNOWN: i32 = 0;

/// Label an `nvtxResourceAttributes_t::identifierType` tag.
///
/// Total by construction: the core `nvtxResourceGenericType_t` set gets a static
/// label, and *everything else* — CUDA and other extension classes, and any
/// value a future NVTX version invents — passes through as
/// `"<identifier_type {n}>"`. The raw integer is the only thing interpolated, so
/// a stream cannot make an unrecognized type render as a recognized one, and no
/// semantics are fabricated for a tag whose meaning we do not know.
pub(crate) fn label_identifier_type(identifier_type: i32) -> String {
    // Confirmed against the pixi-pinned nvtx-c headers (`nvToolsExt.h`,
    // `nvtxResourceGenericType_t`), the same headers `nvtx-injection`'s bindgen
    // build reads.
    let label = match identifier_type {
        TYPE_UNKNOWN => "unknown",
        _ if identifier_type == resource_type(CLASS_GENERIC, 1) => "generic pointer",
        _ if identifier_type == resource_type(CLASS_GENERIC, 2) => "generic handle",
        _ if identifier_type == resource_type(CLASS_GENERIC, 3) => "native thread",
        _ if identifier_type == resource_type(CLASS_GENERIC, 4) => "posix thread",
        _ => return format!("<identifier_type {identifier_type}>"),
    };
    label.to_owned()
}

/// A `ResourceCreate` awaiting its matching `ResourceDestroy`.
struct OpenResource {
    /// Recovered from the create, because the destroy carries no domain.
    domain: u64,
    /// Resolved at create time against the pass-1 tables, which are already
    /// complete by the time replay begins.
    name: String,
    identifier_type: i32,
    start: TimeUnixNanoSec,
}

impl OpenResource {
    /// Close this resource, forming the reconstructed lifespan.
    ///
    /// `end` is clamped up to `start`, matching the range reconstructions, so an
    /// inverted pair yields a zero-duration span rather than an underflowing one.
    fn close(self, end: TimeUnixNanoSec, synthetic_end: bool) -> NvtxSpan {
        NvtxSpan {
            domain: self.domain,
            // Resource create/destroy carry no thread id; a resource is not
            // owned by the thread that happened to announce it.
            thread_id: None,
            name: self.name,
            // `nvtxResourceAttributes_t` has no category, color, or payload —
            // it is not an event-attribute struct. Leaving these empty is the
            // honest reading; inventing them would be fabrication.
            category: None,
            color: None,
            payload: None,
            start: self.start,
            end: end.max(self.start),
            kind: SpanKind::Resource,
            identifier_type_label: Some(label_identifier_type(self.identifier_type)),
            // Resource lifespans do not nest — there is no resource stack.
            parent: None,
            synthetic_end,
        }
    }
}

/// The set of currently-open resource lifespans, keyed by handle alone.
#[derive(Default)]
pub(crate) struct Resources {
    open: HashMap<u64, OpenResource>,
}

impl Resources {
    /// Record a `ResourceCreate` under its already-resolved `name`.
    pub(crate) fn create(
        &mut self,
        handle: u64,
        domain: u64,
        name: String,
        identifier_type: i32,
        start: TimeUnixNanoSec,
    ) {
        let open = OpenResource {
            domain,
            name,
            identifier_type,
            start,
        };
        if self.open.insert(handle, open).is_some() {
            warn!(
                "nvtx resource handle 0x{handle:X} was recreated before it was destroyed; \
                 dropping the earlier create"
            );
        }
    }

    /// Close the resource matching `handle`, if one is open.
    ///
    /// Matching is on the handle alone — see the module docs. Returns `None` for
    /// an orphan destroy, which is the *normal* case for a resource created
    /// before capture attached, so it is logged and skipped rather than fatal.
    pub(crate) fn destroy(&mut self, handle: u64, end: TimeUnixNanoSec) -> Option<NvtxSpan> {
        let Some(open) = self.open.remove(&handle) else {
            warn!(
                "orphan nvtx resource destroy for handle 0x{handle:X} with no open create; skipping"
            );
            return None;
        };
        Some(open.close(end, false))
    }

    /// Close every resource still open at the end of the trace.
    ///
    /// Spans come back ordered by start timestamp (then by handle) so a stream
    /// with several leaked resources still reconstructs deterministically.
    pub(crate) fn close_at_trace_end(&mut self, trace_end: TimeUnixNanoSec) -> Vec<NvtxSpan> {
        let mut leaked: Vec<(u64, OpenResource)> = self.open.drain().collect();
        leaked.sort_by_key(|(handle, open)| (open.start, *handle));

        leaked
            .into_iter()
            .map(|(handle, open)| {
                warn!(
                    "nvtx resource handle 0x{handle:X} was never destroyed; closing it at \
                     trace end ({trace_end})"
                );
                open.close(trace_end, true)
            })
            .collect()
    }
}
