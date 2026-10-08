// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Verbatim, application-agnostic NVTX event vocabulary.
//!
//! The bridge converts owned injection records into [`NvtxEvent`]s. Domain,
//! category, resource, and registered-string handles remain raw integers. The
//! analyzer resolves their names from the event stream later.
//!
//! [`NvtxEvent`] also names its Quent event stream.
//! This crate is scheduled for removal once Quent generates NVTX event
//! definitions from its schema. See [#76](https://github.com/rapidsai/quent/issues/76).

mod attributes;
mod payload;

use quent_events::EventPayload;

pub use attributes::{NvtxColor, NvtxEventAttributes, NvtxMessage};
pub use payload::{NvtxPayload, NvtxPayloadValue, PayloadExtensionEvent};

use serde::{Deserialize, Serialize};

/// A verbatim NVTX core event.
///
/// Every variant mirrors one core NVTX call kind. Handles are raw integers,
/// captured with no resolution. The default (NULL) domain is represented as a
/// `domain` of `0`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum NvtxEvent {
    /// `nvtxDomainRangePushEx` — open a nested (per-thread) range.
    RangePush {
        /// Raw domain handle (`0` = default domain).
        domain: u64,
        /// Raw OS thread id (same id space as [`NvtxEvent::NameThread`]).
        thread_id: u32,
        /// Captured event attributes (message, color, category, payload).
        attributes: NvtxEventAttributes,
    },
    /// `nvtxDomainRangePop` — close the most recent push on this thread.
    RangePop {
        /// Raw domain handle (`0` = default domain).
        domain: u64,
        /// Raw OS thread id (same id space as [`NvtxEvent::NameThread`]).
        thread_id: u32,
    },
    /// `nvtxDomainRangeStartEx` — open a process-wide range keyed by id.
    RangeStart {
        /// Raw domain handle (`0` = default domain).
        domain: u64,
        /// Raw `nvtxRangeId_t` correlating start and end.
        range_id: u64,
        /// Captured event attributes (message, color, category, payload).
        attributes: NvtxEventAttributes,
    },
    /// `nvtxDomainRangeEnd` — close the range with the matching id.
    RangeEnd {
        /// Raw domain handle (`0` = default domain).
        domain: u64,
        /// Raw `nvtxRangeId_t` correlating start and end.
        range_id: u64,
    },
    /// `nvtxDomainMarkEx` — an instantaneous marker.
    Mark {
        /// Raw domain handle (`0` = default domain).
        domain: u64,
        /// Captured event attributes (message, color, category, payload).
        attributes: NvtxEventAttributes,
    },
    /// `nvtxDomainCreate` — create a named domain.
    DomainCreate {
        /// Raw domain handle assigned by NVTX.
        domain: u64,
        /// The domain's name.
        name: String,
    },
    /// `nvtxDomainDestroy` — destroy a domain.
    DomainDestroy {
        /// Raw domain handle being destroyed.
        domain: u64,
    },
    /// `nvtxDomainRegisterString` — register a string, returning a handle.
    RegisterString {
        /// Raw domain handle the string is registered against.
        domain: u64,
        /// Raw registered-string handle assigned by NVTX.
        handle: u64,
        /// The registered string value.
        string: String,
    },
    /// `nvtxDomainNameCategory` — name a category within a domain.
    NameCategory {
        /// Raw domain handle the category belongs to.
        domain: u64,
        /// Raw category id (namespaced by `domain` in the analyzer).
        category: u32,
        /// The category's name.
        name: String,
    },
    /// `nvtxNameOsThread` — name an OS thread.
    NameThread {
        /// Raw OS thread id.
        thread_id: u32,
        /// The thread's name.
        name: String,
    },
    /// `nvtxDomainResourceCreate` — associate a resource with a handle.
    ResourceCreate {
        /// Raw domain handle the resource belongs to.
        domain: u64,
        /// Raw resource handle assigned by NVTX.
        handle: u64,
        /// Raw `identifierType` tag from `nvtxResourceAttributes_t`.
        identifier_type: i32,
        /// Raw identifier value (union member captured as bits).
        identifier: u64,
        /// Optional resource name (immediate string or registered handle).
        message: Option<NvtxMessage>,
    },
    /// `nvtxDomainResourceDestroy` — release a resource handle.
    ResourceDestroy {
        /// Raw resource handle being destroyed.
        handle: u64,
    },
}

impl EventPayload for NvtxEvent {
    const NAME: &'static str = "NvtxEvent";
}
