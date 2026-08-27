// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Pass-1 handle-resolution tables and the placeholder policy.
//!
//! NVTX captures every name as a raw integer handle, and the registration giving
//! a handle meaning may appear anywhere in the stream — including after the
//! events using it. Resolution therefore needs a prior scan over the whole
//! stream, and that scan is order-independent so a forward reference resolves
//! exactly as a backward one does.
//!
//! Two keying rules matter most, because getting either wrong produces
//! plausible-but-wrong labels rather than an error:
//!
//! - registered strings key on `(domain, handle)`, never the bare handle — NVTX
//!   registers strings per domain, so one handle value names different strings
//!   in different domains;
//! - category names key on `(domain, category)`, never globally — small ids like
//!   `1` collide across domains constantly.
//!
//! Anything unresolved gets a placeholder that is a pure function of the raw id,
//! so the same stream always renders the same labels and an unresolved name is
//! visibly bracketed rather than passing as a real one.

use std::collections::BTreeSet;

use rustc_hash::FxHashMap as HashMap;

use nvtx_bridge::NvtxEventEntity;
use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};
use quent_events::Event;
use quent_time::TimeUnixNanoSec;

use crate::span::{NvtxCategory, NvtxDomain, NvtxThread, category_id};

/// Label for the NVTX default (NULL) domain when nothing names it.
///
/// Domain `0` is legitimately unnamed — every uninstrumented `nvtxMark` lands
/// there — so it gets a clean label rather than a bracketed placeholder.
const DEFAULT_DOMAIN_NAME: &str = "default domain";

/// Label for an event that carried no message at all.
const UNNAMED_MESSAGE: &str = "<unnamed>";

/// Placeholder for a non-zero domain handle that was never created.
fn unresolved_domain_name(domain: u64) -> String {
    format!("<domain 0x{domain:X}>")
}

/// Placeholder for a registered-string handle that was never registered.
fn unregistered_string_name(handle: u64) -> String {
    format!("<unregistered string 0x{handle:X}>")
}

/// Placeholder for a non-zero category that was never named.
///
/// The domain is in the label because a category id alone is ambiguous.
fn unnamed_category_name(domain: u64, category: u32) -> String {
    format!("<category {category} @ domain 0x{domain:X}>")
}

/// Label for a thread that emitted events but was never named.
///
/// Unnamed threads are the norm, so this is a clean label rather than a
/// bracketed placeholder.
fn unnamed_thread_name(thread_id: u32) -> String {
    format!("thread {thread_id}")
}

/// When a domain existed, as far as the stream shows.
#[derive(Debug, Clone, Copy)]
struct DomainLifespan {
    /// The earliest timestamp at which anything referenced this domain.
    ///
    /// Surfaced in its own right rather than substituted for [`Self::created`]:
    /// a domain created before capture started is still observable, but when it
    /// was *first seen* is a different fact from when it was created.
    first_seen: TimeUnixNanoSec,
    /// The `DomainCreate` timestamp, when one was captured.
    created: Option<TimeUnixNanoSec>,
    /// The `DomainDestroy` timestamp, when one was captured.
    destroyed: Option<TimeUnixNanoSec>,
}

/// Everything pass 1 learns from the stream, ready for pass 2 to resolve against.
///
/// Built by a single scan. The lifespan bounds fold with `min`/`max`, so they
/// are order-independent outright; the four name tables are last-write-wins, so
/// a handle registered twice under different names resolves to the later one.
/// That is still deterministic because [`Self::build`] is handed the
/// timestamp-ordered stream, not the arrival-ordered one — the ordering is what
/// makes "later" well defined.
#[derive(Debug, Default)]
pub(crate) struct ResolutionTables {
    /// `DomainCreate` names, keyed by raw domain handle.
    domain_names: HashMap<u64, String>,
    /// Lifespan per domain, including domains only ever referenced.
    domain_lifespans: HashMap<u64, DomainLifespan>,
    /// `RegisterString` values, keyed **per domain**.
    registered_strings: HashMap<(u64, u64), String>,
    /// `NameCategory` names, keyed **per domain** — never globally.
    category_names: HashMap<(u64, u32), String>,
    /// `NameThread` names, keyed by raw OS thread id.
    thread_names: HashMap<u32, String>,
    /// Every non-zero `(domain, category)` the stream referenced or named.
    categories_seen: BTreeSet<(u64, u32)>,
    /// Every OS thread id the stream referenced or named.
    threads_seen: BTreeSet<u32>,
}

impl ResolutionTables {
    /// Scan the whole stream and build every lookup table (pass 1).
    ///
    /// Order-independent by construction, which is what makes a `RegisterString`
    /// that arrives *after* the range using its handle resolve correctly.
    pub(crate) fn build(events: &[Event<NvtxEventEntity>]) -> Self {
        let mut tables = Self::default();
        for event in events {
            tables.observe(event.timestamp, &event.data.0);
        }
        tables
    }

    /// Fold one event into the tables.
    fn observe(&mut self, timestamp: TimeUnixNanoSec, event: &NvtxEvent) {
        match event {
            NvtxEvent::RangePush {
                domain,
                thread_id,
                attributes,
            } => {
                self.see_domain(*domain, timestamp);
                self.threads_seen.insert(*thread_id);
                self.see_attributes(*domain, attributes);
            }
            NvtxEvent::RangePop { domain, thread_id } => {
                self.see_domain(*domain, timestamp);
                self.threads_seen.insert(*thread_id);
            }
            NvtxEvent::RangeStart {
                domain, attributes, ..
            }
            | NvtxEvent::Mark { domain, attributes } => {
                self.see_domain(*domain, timestamp);
                self.see_attributes(*domain, attributes);
            }
            NvtxEvent::RangeEnd { domain, .. } => self.see_domain(*domain, timestamp),
            NvtxEvent::DomainCreate { domain, name } => {
                let lifespan = self.lifespan(*domain, timestamp);
                // `min` rather than assignment: a stream that somehow repeats a
                // creation still folds to one deterministic answer.
                lifespan.created = Some(lifespan.created.unwrap_or(timestamp).min(timestamp));
                self.domain_names.insert(*domain, name.clone());
            }
            NvtxEvent::DomainDestroy { domain } => {
                let lifespan = self.lifespan(*domain, timestamp);
                lifespan.destroyed = Some(lifespan.destroyed.unwrap_or(timestamp).max(timestamp));
            }
            NvtxEvent::RegisterString {
                domain,
                handle,
                string,
            } => {
                self.see_domain(*domain, timestamp);
                // Keyed by `(domain, handle)`: the same handle value in another
                // domain is a different string.
                self.registered_strings
                    .insert((*domain, *handle), string.clone());
            }
            NvtxEvent::NameCategory {
                domain,
                category,
                name,
            } => {
                self.see_domain(*domain, timestamp);
                // Naming the "no category" sentinel is meaningless, so it never
                // enters the tables or the model view.
                if let Some(category) = category_id(*category) {
                    self.categories_seen.insert((*domain, category));
                    self.category_names
                        .insert((*domain, category), name.clone());
                }
            }
            NvtxEvent::NameThread { thread_id, name } => {
                self.threads_seen.insert(*thread_id);
                self.thread_names.insert(*thread_id, name.clone());
            }
            NvtxEvent::ResourceCreate { domain, .. } => self.see_domain(*domain, timestamp),
            // `ResourceDestroy` carries neither a domain nor a message, so it
            // contributes nothing to resolution.
            NvtxEvent::ResourceDestroy { .. } => {}
        }
    }

    /// The lifespan entry for `domain`, widening its first-seen bound.
    fn lifespan(&mut self, domain: u64, timestamp: TimeUnixNanoSec) -> &mut DomainLifespan {
        let lifespan = self
            .domain_lifespans
            .entry(domain)
            .or_insert(DomainLifespan {
                first_seen: timestamp,
                created: None,
                destroyed: None,
            });
        lifespan.first_seen = lifespan.first_seen.min(timestamp);
        lifespan
    }

    /// Record that `domain` exists, without learning anything else about it.
    fn see_domain(&mut self, domain: u64, timestamp: TimeUnixNanoSec) {
        let _ = self.lifespan(domain, timestamp);
    }

    /// Record the category an event referenced, if it referenced one.
    fn see_attributes(&mut self, domain: u64, attributes: &NvtxEventAttributes) {
        if let Some(category) = category_id(attributes.category) {
            self.categories_seen.insert((domain, category));
        }
    }

    /// Render a captured message as a display name.
    ///
    /// Registered handles resolve against `domain`; an unregistered handle falls
    /// back to a placeholder that surfaces the raw handle rather than failing.
    pub(crate) fn resolve_message(&self, domain: u64, message: &Option<NvtxMessage>) -> String {
        match message {
            Some(NvtxMessage::String(text)) => text.clone(),
            Some(NvtxMessage::RegisteredHandle(handle)) => self
                .registered_strings
                .get(&(domain, *handle))
                .cloned()
                .unwrap_or_else(|| unregistered_string_name(*handle)),
            None => UNNAMED_MESSAGE.to_owned(),
        }
    }

    /// Render a domain handle as a display name.
    pub(crate) fn resolve_domain(&self, domain: u64) -> String {
        if let Some(name) = self.domain_names.get(&domain) {
            return name.clone();
        }
        // Domain 0 is legitimately unnamed; any other unresolved handle is a
        // genuine gap and says so.
        if domain == 0 {
            DEFAULT_DOMAIN_NAME.to_owned()
        } else {
            unresolved_domain_name(domain)
        }
    }

    /// Render a non-zero category as a display name, namespaced by its domain.
    fn category_name(&self, domain: u64, category: u32) -> String {
        self.category_names
            .get(&(domain, category))
            .cloned()
            .unwrap_or_else(|| unnamed_category_name(domain, category))
    }

    /// Render a category as a display name, namespaced by its domain.
    ///
    /// `None` for category `0`: an absence rather than an unresolved reference,
    /// so it gets no placeholder.
    pub(crate) fn resolve_category(&self, domain: u64, category: u32) -> Option<String> {
        category_id(category).map(|category| self.category_name(domain, category))
    }

    /// Render an OS thread id as a display name.
    pub(crate) fn resolve_thread(&self, thread_id: u32) -> String {
        self.thread_names
            .get(&thread_id)
            .cloned()
            .unwrap_or_else(|| unnamed_thread_name(thread_id))
    }

    /// Every domain the stream mentioned, resolved and ordered by handle.
    pub(crate) fn domain_records(&self) -> Vec<NvtxDomain> {
        let mut records: Vec<NvtxDomain> = self
            .domain_lifespans
            .iter()
            .map(|(&domain, lifespan)| NvtxDomain {
                domain,
                name: self.resolve_domain(domain),
                created: lifespan.created,
                first_seen: lifespan.first_seen,
                destroyed: lifespan.destroyed,
            })
            .collect();
        // A `HashMap` iteration order is unspecified; sort so repeated builds of
        // the same stream produce identical models.
        records.sort_by_key(|record| record.domain);
        records
    }

    /// Every OS thread the stream mentioned, resolved and ordered by id.
    pub(crate) fn thread_records(&self) -> Vec<NvtxThread> {
        self.threads_seen
            .iter()
            .map(|&thread_id| NvtxThread {
                thread_id,
                name: self.resolve_thread(thread_id),
            })
            .collect()
    }

    /// Every non-zero category the stream mentioned, ordered by `(domain, id)`.
    pub(crate) fn category_records(&self) -> Vec<NvtxCategory> {
        self.categories_seen
            .iter()
            .map(|&(domain, category)| NvtxCategory {
                domain,
                category,
                // `categories_seen` never holds category `0`, so this needs no
                // sentinel round-trip through `resolve_category`.
                name: self.category_name(domain, category),
            })
            .collect()
    }
}
