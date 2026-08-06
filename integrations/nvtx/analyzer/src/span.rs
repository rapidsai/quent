// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The reconstruction core's own plain span types.
//!
//! Hand-written structs, not framework types. Keeping our own is what lets a
//! zero-duration span, or one whose close was never captured, be representable
//! here — the shared framework would reject or panic on both.

use nvtx_events::{NvtxColor, NvtxPayload};
use quent_time::TimeUnixNanoSec;

/// A stable handle to an [`NvtxSpan`] within one reconstructed model.
///
/// The index into the owning model's span list. Only meaningful against the
/// model it was produced from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SpanId(pub usize);

/// Which NVTX construct a span was reconstructed from, together with the fields
/// only that construct has.
///
/// The kind-conditional attributes live in the variants rather than beside them:
/// a process-wide range has no thread, a resource lifespan has no parent, and
/// neither range kind has an identifier type. Flat `Option` fields would leave
/// those rules stated only in prose, and admit the combinations that contradict
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// A per-thread nested range: `nvtxDomainRangePushEx` / `nvtxDomainRangePop`.
    PushPop {
        /// Raw OS thread id — half of the `(thread, domain)` nesting key, so
        /// always known for this kind.
        thread_id: u32,
        /// The enclosing push on the same stack, or `None` when this one was
        /// outermost on its thread.
        parent: Option<SpanId>,
    },
    /// A process-wide range keyed by id: `nvtxDomainRangeStartEx` / `nvtxDomainRangeEnd`.
    ///
    /// Carries no thread: the id comes from a process-global counter and the end
    /// may be emitted from any thread.
    StartEnd,
    /// A resource lifespan: `nvtxDomainResourceCreate` / `nvtxDomainResourceDestroy`.
    Resource {
        /// Raw `nvtxResourceAttributes_t::identifierType`, undecoded — see
        /// [`identifier_type_label`](Self::identifier_type_label).
        identifier_type: i32,
    },
}

impl SpanKind {
    /// The OS thread this span ran on, for the one kind that records it.
    pub fn thread_id(self) -> Option<u32> {
        match self {
            Self::PushPop { thread_id, .. } => Some(thread_id),
            Self::StartEnd | Self::Resource { .. } => None,
        }
    }

    /// The enclosing span, for the one kind that nests.
    pub fn parent(self) -> Option<SpanId> {
        match self {
            Self::PushPop { parent, .. } => parent,
            Self::StartEnd | Self::Resource { .. } => None,
        }
    }

    /// What kind of thing a [`Resource`](Self::Resource) span identifies.
    ///
    /// A core/generic NVTX resource type gets a static label; an unrecognized or
    /// CUDA-extension type passes through as `"<identifier_type {n}>"` rather
    /// than being guessed at.
    pub fn identifier_type_label(self) -> Option<String> {
        match self {
            Self::Resource { identifier_type } => Some(label_identifier_type(identifier_type)),
            Self::PushPop { .. } | Self::StartEnd => None,
        }
    }
}

/// Label an `nvtxResourceAttributes_t::identifierType` tag.
///
/// `nvToolsExt.h` composes an identifier type as
/// `NVTX_RESOURCE_MAKE_TYPE(CLASS, INDEX) = (CLASS << 16) | INDEX`, and
/// `NVTX_RESOURCE_CLASS_GENERIC` is `1` — so the core `nvtxResourceGenericType_t`
/// set is `0x0001_0001`..=`0x0001_0004`. Other classes (CUDA, CUDA runtime,
/// OpenCL, D3D, sync) are extension surfaces this crate deliberately does not
/// interpret.
///
/// Total by construction: everything outside the core set passes through as
/// `"<identifier_type {n}>"`, interpolating only the raw integer, so an
/// unrecognized type can never render as a recognized one.
fn label_identifier_type(identifier_type: i32) -> String {
    // Confirmed against the pixi-pinned nvtx-c headers, the same ones
    // `nvtx-injection`'s bindgen build reads.
    let label = match identifier_type {
        // `NVTX_RESOURCE_TYPE_UNKNOWN`, also what the capture layer records for
        // an attribute struct too short to contain the field — a legitimate
        // "not stated" rather than an anomaly.
        0x0000_0000 => "unknown",
        0x0001_0001 => "generic pointer",
        0x0001_0002 => "generic handle",
        0x0001_0003 => "native thread",
        0x0001_0004 => "posix thread",
        _ => return format!("<identifier_type {identifier_type}>"),
    };
    label.to_owned()
}

/// Map a raw category id onto the presence it encodes.
///
/// Category `0` is NVTX's "no category" sentinel — an absence, not an id. Every
/// consumer goes through here so the rule is stated once. `nvtx-events` would
/// be the better home, since it already models `color`, `message`, and
/// `payload` as `Option` and `category` is the one attribute left as a magic
/// number.
pub(crate) const fn category_id(raw: u32) -> Option<u32> {
    if raw == 0 { None } else { Some(raw) }
}

/// A reconstructed NVTX interval.
///
/// The open is always known — it is what carried the name and attributes. The
/// close may not be, and then [`end`](Self::end) is `None`: this layer resolves
/// what the stream referenced and substitutes nothing for what it never said.
#[derive(Debug, Clone, PartialEq)]
pub struct NvtxSpan {
    /// Raw domain handle (`0` = default domain).
    pub domain: u64,
    /// Resolved message, or a placeholder when the handle was never registered.
    pub name: String,
    /// Raw category id (`None` when the event carried category `0` = none).
    pub category: Option<u32>,
    /// Verbatim color attribute, undecoded.
    pub color: Option<NvtxColor>,
    /// Verbatim payload value, undecoded.
    pub payload: Option<NvtxPayload>,
    pub start: TimeUnixNanoSec,
    /// When the close was observed, or `None` when it never was.
    ///
    /// Not "unknown yet" — reconstruction is complete before a model exists, so
    /// `None` states that the stream carried no close for this open. It happens
    /// routinely: a capture detaches while work is still running, or a later
    /// open reuses this one's key. Whether to bound such a span, and at what,
    /// is the consuming analysis's decision — [`NvtxModel::trace_end`] is where
    /// observation stopped.
    ///
    /// When present, always `>= start`. That is not enforced by clamping; it
    /// follows from replay running in total timestamp order.
    ///
    /// [`NvtxModel::trace_end`]: crate::NvtxModel::trace_end
    pub end: Option<TimeUnixNanoSec>,
    /// Which NVTX construct this span was reconstructed from, and its
    /// kind-specific fields.
    pub kind: SpanKind,
}

impl NvtxSpan {
    /// The measured duration in nanoseconds, or `None` when the close was never
    /// captured.
    pub fn duration(&self) -> Option<u64> {
        Some(self.end?.saturating_sub(self.start))
    }
}

/// A reconstructed `nvtxDomainMarkEx` instant: a timestamp with no duration.
///
/// No thread: `nvtxDomainMarkEx` does not report one, so a mark says when
/// something happened but never where.
#[derive(Debug, Clone, PartialEq)]
pub struct NvtxMark {
    /// Raw domain handle (`0` = default domain).
    pub domain: u64,
    /// Resolved message, or a placeholder when the handle was never registered.
    pub name: String,
    /// Raw category id (`None` when the event carried category `0` = none).
    pub category: Option<u32>,
    /// Verbatim color attribute, undecoded.
    pub color: Option<NvtxColor>,
    /// Verbatim payload value, undecoded.
    pub payload: Option<NvtxPayload>,
    /// When the mark was emitted.
    pub timestamp: TimeUnixNanoSec,
}

/// A domain the stream referenced, with its resolved name and lifespan.
///
/// Present for every domain *mentioned* by the stream, not only those with a
/// captured `nvtxDomainCreate` — a domain created before capture began still
/// groups the events that name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NvtxDomain {
    /// Raw domain handle (`0` = default domain).
    pub domain: u64,
    /// Resolved name, or the placeholder for an uncreated handle.
    pub name: String,
    /// The `nvtxDomainCreate` timestamp, or `None` when no creation was
    /// captured — the domain already existed when capture attached.
    ///
    /// Kept separate from [`first_seen`](Self::first_seen) rather than
    /// substituted into it: the two answer different questions, and a domain
    /// created before the capture would otherwise report a creation time that is
    /// really just the first event mentioning it.
    pub created: Option<TimeUnixNanoSec>,
    /// The earliest timestamp at which anything referenced this domain.
    ///
    /// Always known, and an upper bound on the real creation time.
    pub first_seen: TimeUnixNanoSec,
    /// The `nvtxDomainDestroy` timestamp, if one was captured.
    pub destroyed: Option<TimeUnixNanoSec>,
}

/// An OS thread the stream referenced, with its resolved name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NvtxThread {
    /// Raw OS thread id (the `nvtxNameOsThread` id space).
    pub thread_id: u32,
    /// Resolved name, or `"thread {id}"` when the thread was never named.
    pub name: String,
}

/// A category the stream referenced, namespaced by its owning domain.
///
/// The `(domain, category)` pair *is* the identity: NVTX category ids are only
/// unique within a domain, so a globally-keyed view would silently merge
/// unrelated categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NvtxCategory {
    /// Raw domain handle the category belongs to.
    pub domain: u64,
    /// Raw category id (never `0` — that is the "no category" sentinel).
    pub category: u32,
    /// Resolved name, or the placeholder for an unnamed category.
    pub name: String,
}
