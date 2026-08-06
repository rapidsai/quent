// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resource lifespan reconstruction — the third match key.
//!
//! The interval between `nvtxDomainResourceCreate` and its destroy, modelled as
//! an [`NvtxSpan`] with [`SpanKind::Resource`].
//!
//! **The match key is the handle alone**, not `(domain, handle)` — forced by the
//! vocabulary, since
//! [`NvtxEvent::ResourceDestroy`](nvtx_events::NvtxEvent::ResourceDestroy)
//! carries only a handle because the underlying NVTX call does. Keying on the
//! pair would not fail loudly: every destroy would miss its create and every
//! resource would silently reconstruct as a leak with no end. The domain is
//! recovered from the create instead.
//!
//! A create says a handle exists and what it is called — nothing about its size
//! or occupancy — so nothing further is inferred.

use rustc_hash::FxHashMap as HashMap;

use quent_time::TimeUnixNanoSec;
use tracing::{debug, warn};

use crate::span::{NvtxSpan, SpanKind};

/// A `ResourceCreate` awaiting its matching `ResourceDestroy`.
struct OpenResource {
    /// Recovered from the create, because the destroy carries no domain.
    domain: u64,
    name: String,
    identifier_type: i32,
    start: TimeUnixNanoSec,
}

impl OpenResource {
    /// Close this resource, with `end` left `None` when no destroy was observed.
    ///
    /// `end >= start` is a precondition established by timestamp-ordered
    /// replay, matching the range reconstructions.
    fn close(self, end: Option<TimeUnixNanoSec>) -> NvtxSpan {
        debug_assert!(
            end.is_none_or(|end| end >= self.start),
            "replay is timestamp-ordered, so a destroy cannot precede its create"
        );
        NvtxSpan {
            domain: self.domain,
            name: self.name,
            // `nvtxResourceAttributes_t` is not an event-attribute struct; it
            // has no category, color, or payload to carry.
            category: None,
            color: None,
            payload: None,
            start: self.start,
            end,
            kind: SpanKind::Resource {
                identifier_type: self.identifier_type,
            },
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
    ///
    /// Returns a span when this create *displaces* a lifespan still open under
    /// the same handle. The displaced lifespan keeps everything the create
    /// stated and ends `None`, rather than being discarded — the create was
    /// observed, only its destroy never arrived. The caller tallies the reuse.
    pub(crate) fn create(
        &mut self,
        handle: u64,
        domain: u64,
        name: String,
        identifier_type: i32,
        start: TimeUnixNanoSec,
    ) -> Option<NvtxSpan> {
        let open = OpenResource {
            domain,
            name,
            identifier_type,
            start,
        };
        let displaced = self.open.insert(handle, open)?;
        warn!(
            "nvtx resource handle 0x{handle:X} was recreated at {start} before it was destroyed; \
             the earlier lifespan has no end"
        );
        Some(displaced.close(None))
    }

    /// Close the resource matching `handle`, if one is open.
    ///
    /// Returns `None` for an orphan destroy — the *normal* case for a resource
    /// created before capture attached. The caller counts those.
    pub(crate) fn destroy(&mut self, handle: u64, end: TimeUnixNanoSec) -> Option<NvtxSpan> {
        let Some(open) = self.open.remove(&handle) else {
            // `debug!`, not `warn!`, because this is routine: logging it as an
            // anomaly would bury the genuine ones.
            debug!(
                "orphan nvtx resource destroy for handle 0x{handle:X} with no open create; skipping"
            );
            return None;
        };
        Some(open.close(Some(end)))
    }

    /// Emit every resource still open when the stream ran out, ending `None`.
    ///
    /// Ordered by `(start, handle)` so several leaked resources still
    /// reconstruct deterministically out of an unordered `HashMap`.
    pub(crate) fn drain_unclosed(&mut self) -> Vec<NvtxSpan> {
        let mut leaked: Vec<(u64, OpenResource)> = self.open.drain().collect();
        leaked.sort_by_key(|(handle, open)| (open.start, *handle));

        // One `warn!` for the count, per-handle detail at `debug!`, so a stream
        // leaking many resources does not emit one warning each.
        if !leaked.is_empty() {
            warn!(
                "{} nvtx resource(s) were never destroyed; their lifespans have no end",
                leaked.len()
            );
        }

        leaked
            .into_iter()
            .map(|(handle, open)| {
                debug!("nvtx resource handle 0x{handle:X} was never destroyed");
                open.close(None)
            })
            .collect()
    }
}
