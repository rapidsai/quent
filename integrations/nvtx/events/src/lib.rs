// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! NVTX event types for the Quent NVTX integration.
//!
//! These types represent raw NVTX API calls as captured by the injection
//! library. They are serialized as-is and interpreted by the analyzer.

mod attributes;
mod payload;

pub use attributes::{NvtxAttributes, NvtxMessage, NvtxPayload};
pub use payload::{
    EnumRegister, MarkPayload, PopPayload, PushPayload, RangeEndPayload, RangeStartPayload,
    RawPayloadData, RawPayloadEnum, RawPayloadSchema, SchemaRegister,
};

use serde::{Deserialize, Serialize};

/// A raw NVTX API event captured by the injection library.
///
/// Each variant corresponds to one NVTX API call. The injection performs
/// no interpretation — all fields are the verbatim arguments from the
/// original call. The analyzer reconstructs traces, FSMs, and handle
/// mappings from the event stream.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum NvtxEvent {
    // Push/Pop ranges (thread-scoped, stack-based)
    Push(Push),
    Pop(Pop),

    // Start/End ranges (process-scoped, ID-based)
    RangeStart(RangeStart),
    RangeEnd(RangeEnd),

    // Marks (instant events)
    Mark(Mark),

    // Domain lifecycle
    DomainCreate(DomainCreate),
    DomainDestroy(DomainDestroy),

    // Registered strings
    RegisterString(RegisterString),

    // Category naming
    NameCategory(NameCategory),

    // Thread naming
    NameThread(NameThread),

    // Resource lifecycle
    ResourceCreate(ResourceCreate),
    ResourceDestroy(ResourceDestroy),

    // Payload extension (not yet emitted by the injection — Phase 5)
    SchemaRegister(SchemaRegister),
    EnumRegister(EnumRegister),
    PushPayload(PushPayload),
    PopPayload(PopPayload),
    RangeStartPayload(RangeStartPayload),
    RangeEndPayload(RangeEndPayload),
    MarkPayload(MarkPayload),
}

// Core push/pop
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Push {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub attributes: Option<NvtxAttributes>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Pop {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
}

// Start/End ranges
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RangeStart {
    pub range_handle_id: u64,
    pub domain_handle_id: Option<u64>,
    pub attributes: Option<NvtxAttributes>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RangeEnd {
    pub range_handle_id: u64,
    pub domain_handle_id: Option<u64>,
}

// Marks
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Mark {
    pub thread_id: u64,
    pub domain_handle_id: Option<u64>,
    pub attributes: Option<NvtxAttributes>,
}

// Domains
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DomainCreate {
    pub domain_handle_id: u64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DomainDestroy {
    pub domain_handle_id: u64,
}

// Registered strings
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RegisterString {
    pub domain_handle_id: Option<u64>,
    pub string_handle_id: u64,
    pub value: String,
}

// Categories
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NameCategory {
    pub domain_handle_id: Option<u64>,
    pub category_id: u32,
    pub name: String,
}

// Thread naming
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NameThread {
    pub os_thread_id: u32,
    pub name: String,
}

// Resources
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ResourceCreate {
    pub domain_handle_id: Option<u64>,
    pub resource_handle_id: u64,
    pub attributes: NvtxResourceAttributes,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ResourceDestroy {
    pub resource_handle_id: u64,
}

/// Attributes from `nvtxResourceAttributes_t`.
///
/// Contains an identifier (which may be a pointer, integer, or OS handle)
/// and a message. The identifier is captured as a raw u64; the analyzer
/// decides how to interpret it.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NvtxResourceAttributes {
    pub identifier: u64,
    pub message: Option<NvtxMessage>,
}
