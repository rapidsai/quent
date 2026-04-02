// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Quent integration for NVTX.
//!
//! Thin wrapper around [`quent_nvtx_injection`] that connects NVTX event capture
//! to Quent's [`EventSender`](quent_instrumentation::EventSender).

use std::fmt::Debug;

use quent_events::Event;
use quent_instrumentation::EventSender;
use serde::Serialize;
use uuid::Uuid;

pub use quent_nvtx_events;
pub use quent_nvtx_events::NvtxEvent;
pub use quent_nvtx_injection;

/// Install the NVTX injection, forwarding events through a Quent
/// [`EventSender`].
///
/// Must be called before the first NVTX API call.
///
/// The application's event type `T` must implement `From<NvtxEvent>` so
/// that raw NVTX events can be wrapped into the application's event enum.
pub fn install<T>(sender: EventSender<T>, session_id: Uuid)
where
    T: From<NvtxEvent> + Serialize + Send + Debug + 'static,
{
    quent_nvtx_injection::install_hook(move |nvtx_event| {
        sender.send(Event::new_now(session_id, T::from(nvtx_event)));
    });
}
