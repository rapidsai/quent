// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Borrowed access to NVTX data, independent of its stored representation.
//!
//! Implementations expose fields without allocating or resolving handles. The
//! reconstruction core only owns names and scalar attributes in its results;
//! it never needs an intermediate collection of native NVTX events.

use nvtx_events::{NvtxColor, NvtxEvent, NvtxEventAttributes, NvtxMessage, NvtxPayload};

/// Access to an event's captured NVTX fields.
///
/// Implement this for application event types to use
/// [`NvtxModelBuilder::build_from`](crate::NvtxModelBuilder::build_from). The
/// view borrows strings from the event; timestamps and source/context selection
/// belong to the caller's event envelope, not to this payload contract.
///
/// Accessors must return the same fields on each call: reconstruction reads the
/// stream twice, first for name resolution and then for range reconstruction.
/// Return `None` for envelope events that carry stream metadata rather than one
/// of the captured NVTX event kinds. Their timestamps still contribute to the
/// reconstructed trace bounds.
pub trait NvtxEventData {
    fn nvtx_event(&self) -> Option<NvtxEventView<'_>>;
}

impl<T: NvtxEventData + ?Sized> NvtxEventData for &T {
    fn nvtx_event(&self) -> Option<NvtxEventView<'_>> {
        (**self).nvtx_event()
    }
}

/// Access to a message without copying immediate strings or resolving handles.
pub trait NvtxMessageData {
    fn nvtx_message(&self) -> NvtxMessageView<'_>;
}

/// Access to the attributes carried by a range or mark.
///
/// Raw color/payload tags and scalar bits must be preserved. Scalar attributes
/// are copied into the result model; only the message needs a borrow.
pub trait NvtxAttributesData {
    fn nvtx_attributes(&self) -> NvtxAttributesView<'_>;
}

/// A captured message borrowed from an event or record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvtxMessageView<'a> {
    String(&'a str),
    RegisteredHandle(u64),
}

/// The captured attributes of a range or mark, with a borrowed message.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct NvtxAttributesView<'a> {
    /// Raw category id (`0` means absent).
    pub category: u32,
    pub color: Option<NvtxColor>,
    pub message: Option<NvtxMessageView<'a>>,
    pub payload: Option<NvtxPayload>,
}

/// Borrowed fields for every captured NVTX core event kind.
///
/// Handles remain raw and names remain unresolved. This is an analysis view,
/// not a wire format. Resource identifiers are exposed even though the current
/// result model only retains their type. Thread IDs use the captured native
/// `u32` representation and can be widened losslessly for OS-identity joins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NvtxEventView<'a> {
    RangePush {
        domain: u64,
        thread_id: u32,
        attributes: NvtxAttributesView<'a>,
    },
    RangePop {
        domain: u64,
        thread_id: u32,
    },
    RangeStart {
        domain: u64,
        range_id: u64,
        attributes: NvtxAttributesView<'a>,
    },
    RangeEnd {
        domain: u64,
        range_id: u64,
    },
    Mark {
        domain: u64,
        attributes: NvtxAttributesView<'a>,
    },
    DomainCreate {
        domain: u64,
        name: &'a str,
    },
    DomainDestroy {
        domain: u64,
    },
    RegisterString {
        domain: u64,
        handle: u64,
        string: &'a str,
    },
    NameCategory {
        domain: u64,
        category: u32,
        name: &'a str,
    },
    NameThread {
        thread_id: u32,
        name: &'a str,
    },
    ResourceCreate {
        domain: u64,
        handle: u64,
        identifier_type: i32,
        identifier: u64,
        message: Option<NvtxMessageView<'a>>,
    },
    ResourceDestroy {
        handle: u64,
    },
}

impl NvtxMessageData for NvtxMessage {
    fn nvtx_message(&self) -> NvtxMessageView<'_> {
        match self {
            Self::String(text) => NvtxMessageView::String(text),
            Self::RegisteredHandle(handle) => NvtxMessageView::RegisteredHandle(*handle),
        }
    }
}

impl NvtxAttributesData for NvtxEventAttributes {
    fn nvtx_attributes(&self) -> NvtxAttributesView<'_> {
        NvtxAttributesView {
            category: self.category,
            color: self.color,
            message: self.message.as_ref().map(NvtxMessageData::nvtx_message),
            payload: self.payload,
        }
    }
}

impl NvtxEventData for NvtxEvent {
    fn nvtx_event(&self) -> Option<NvtxEventView<'_>> {
        // Exhaustive so extending the native vocabulary requires updating the
        // access contract as well as both reconstruction passes.
        Some(match self {
            Self::RangePush {
                domain,
                thread_id,
                attributes,
            } => NvtxEventView::RangePush {
                domain: *domain,
                thread_id: *thread_id,
                attributes: attributes.nvtx_attributes(),
            },
            Self::RangePop { domain, thread_id } => NvtxEventView::RangePop {
                domain: *domain,
                thread_id: *thread_id,
            },
            Self::RangeStart {
                domain,
                range_id,
                attributes,
            } => NvtxEventView::RangeStart {
                domain: *domain,
                range_id: *range_id,
                attributes: attributes.nvtx_attributes(),
            },
            Self::RangeEnd { domain, range_id } => NvtxEventView::RangeEnd {
                domain: *domain,
                range_id: *range_id,
            },
            Self::Mark { domain, attributes } => NvtxEventView::Mark {
                domain: *domain,
                attributes: attributes.nvtx_attributes(),
            },
            Self::DomainCreate { domain, name } => NvtxEventView::DomainCreate {
                domain: *domain,
                name,
            },
            Self::DomainDestroy { domain } => NvtxEventView::DomainDestroy { domain: *domain },
            Self::RegisterString {
                domain,
                handle,
                string,
            } => NvtxEventView::RegisterString {
                domain: *domain,
                handle: *handle,
                string,
            },
            Self::NameCategory {
                domain,
                category,
                name,
            } => NvtxEventView::NameCategory {
                domain: *domain,
                category: *category,
                name,
            },
            Self::NameThread { thread_id, name } => NvtxEventView::NameThread {
                thread_id: *thread_id,
                name,
            },
            Self::ResourceCreate {
                domain,
                handle,
                identifier_type,
                identifier,
                message,
            } => NvtxEventView::ResourceCreate {
                domain: *domain,
                handle: *handle,
                identifier_type: *identifier_type,
                identifier: *identifier,
                message: message.as_ref().map(NvtxMessageData::nvtx_message),
            },
            Self::ResourceDestroy { handle } => NvtxEventView::ResourceDestroy { handle: *handle },
        })
    }
}
