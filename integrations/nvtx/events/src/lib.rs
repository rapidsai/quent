// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Verbatim, Quent-agnostic NVTX event vocabulary.
//!
//! This crate is the shared contract every downstream NVTX crate speaks: the
//! injection cdylib produces [`NvtxEvent`]s and the bridge wraps them for the
//! Quent pipeline. Events are captured **verbatim** — every handle
//! (domain/category/resource/registered-string id) is a raw integer, and no name
//! resolution or payload decoding happens at capture time (D-01). The analyzer
//! resolves handles from the event stream in a later phase.
//!
//! # Separability (D-03)
//!
//! This crate depends on nothing Quent-internal (only `serde` + `uuid`) so it
//! stays cleanly separable and can be offered upstream to NVIDIA/NVTX later. The
//! [`EntityEvent`] contract below is defined locally for exactly this reason —
//! the Quent pipeline's own `EntityEvent` is structurally identical, and the
//! bridge crate adapts [`NvtxEventKind`] into the pipeline.

mod attributes;
mod payload;

pub use attributes::{NvtxColor, NvtxEventAttributes, NvtxMessage};
pub use payload::{NvtxPayload, NvtxPayloadValue, PayloadExtensionEvent};

use serde::{Deserialize, Serialize};

/// The entity-name contract for an event-stream payload type.
///
/// Mirrored locally to keep this vocabulary crate free of Quent-internal
/// dependencies (D-03 separability). It is structurally identical to the Quent
/// pipeline's `quent_events::EntityEvent`; the NVTX bridge adapts
/// [`NvtxEventKind`] into that pipeline.
pub trait EntityEvent {
    /// The name of the entity producing these events.
    const NAME: &'static str;
}

/// A verbatim NVTX core event.
///
/// Every variant mirrors one core NVTX call kind. Handles are raw integers,
/// captured with no resolution (D-01). The default (NULL) domain is represented
/// as a `domain` of `0`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum NvtxEvent {
    /// `nvtxDomainRangePushEx` — open a nested (per-thread) range.
    RangePush {
        /// Raw domain handle (`0` = default domain).
        domain: u64,
        /// Captured event attributes (message, color, category, payload).
        attributes: NvtxEventAttributes,
    },
    /// `nvtxDomainRangePop` — close the most recent push on this thread.
    RangePop {
        /// Raw domain handle (`0` = default domain).
        domain: u64,
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

/// The `Event<T>` payload type carried through the Quent pipeline for the NVTX
/// stream.
///
/// A transparent newtype over [`NvtxEvent`] that carries the [`EntityEvent`]
/// name so captured events can flow through Quent's `EventSender`/exporters. The
/// injection crate produces bare [`NvtxEvent`]s; the bridge converts each into an
/// `NvtxEventKind` via [`From`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NvtxEventKind(pub NvtxEvent);

impl From<NvtxEvent> for NvtxEventKind {
    fn from(event: NvtxEvent) -> Self {
        Self(event)
    }
}

impl EntityEvent for NvtxEventKind {
    const NAME: &'static str = "NvtxEvent";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(event: &NvtxEvent) -> NvtxEvent {
        let json = serde_json::to_string(event).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn every_core_variant_round_trips_through_serde_json() {
        let attrs = NvtxEventAttributes {
            category: 3,
            color: Some(NvtxColor {
                color_type: 1,
                value: 0xFF00_FF00,
            }),
            message: Some(NvtxMessage::String("range".to_owned())),
            payload: None,
        };

        let events = [
            NvtxEvent::RangePush {
                domain: 1,
                attributes: attrs.clone(),
            },
            NvtxEvent::RangePop { domain: 1 },
            NvtxEvent::RangeStart {
                domain: 1,
                range_id: 99,
                attributes: attrs.clone(),
            },
            NvtxEvent::RangeEnd {
                domain: 1,
                range_id: 99,
            },
            NvtxEvent::Mark {
                domain: 1,
                attributes: attrs.clone(),
            },
            NvtxEvent::DomainCreate {
                domain: 1,
                name: "quent".to_owned(),
            },
            NvtxEvent::DomainDestroy { domain: 1 },
            NvtxEvent::RegisterString {
                domain: 1,
                handle: 7,
                string: "registered".to_owned(),
            },
            NvtxEvent::NameCategory {
                domain: 1,
                category: 3,
                name: "io".to_owned(),
            },
            NvtxEvent::NameThread {
                thread_id: 4242,
                name: "worker".to_owned(),
            },
            NvtxEvent::ResourceCreate {
                domain: 1,
                handle: 88,
                identifier_type: 2,
                identifier: 0x1234,
                message: Some(NvtxMessage::RegisteredHandle(7)),
            },
            NvtxEvent::ResourceDestroy { handle: 88 },
        ];

        for event in &events {
            assert_eq!(&round_trip(event), event);
        }
    }

    #[test]
    fn mark_carrying_core_payload_union_round_trips_verbatim() {
        let event = NvtxEvent::Mark {
            domain: 1,
            attributes: NvtxEventAttributes {
                category: 0,
                color: None,
                message: Some(NvtxMessage::String("with-payload".to_owned())),
                payload: Some(NvtxPayload {
                    payload_type: 1,
                    value: NvtxPayloadValue::UnsignedInt64(0xCAFE_F00D),
                }),
            },
        };

        let back = round_trip(&event);
        assert_eq!(back, event);

        // The payload value survives verbatim.
        let NvtxEvent::Mark { attributes, .. } = back else {
            panic!("expected a Mark");
        };
        assert_eq!(
            attributes.payload,
            Some(NvtxPayload {
                payload_type: 1,
                value: NvtxPayloadValue::UnsignedInt64(0xCAFE_F00D),
            })
        );
    }

    #[test]
    fn nvtx_message_variants_round_trip() {
        for message in [
            NvtxMessage::String("immediate".to_owned()),
            NvtxMessage::RegisteredHandle(0xABCD),
        ] {
            let json = serde_json::to_string(&message).expect("serialize");
            let back: NvtxMessage = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, message);
        }
    }

    #[test]
    fn nvtx_event_kind_carries_entity_name_and_wraps_events() {
        assert_eq!(NvtxEventKind::NAME, "NvtxEvent");

        let kind = NvtxEventKind::from(NvtxEvent::RangePop { domain: 0 });
        // Transparent newtype: serializes identically to the inner event.
        let kind_json = serde_json::to_string(&kind).expect("serialize kind");
        let event_json =
            serde_json::to_string(&NvtxEvent::RangePop { domain: 0 }).expect("serialize event");
        assert_eq!(kind_json, event_json);

        let back: NvtxEventKind = serde_json::from_str(&kind_json).expect("deserialize");
        assert_eq!(back, kind);
    }
}
