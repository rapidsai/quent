// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Common event wrapper for all FSM event types.

/// Common wrapper for all FSM events.
///
/// Every generated FSM event type is an alias over `FsmEvent<S>`:
///
/// ```text
/// pub type TaskEvent = FsmEvent<TaskTransition>;
/// ```
///
/// `S` is the transition enum (one variant per state + exit).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FsmEvent<S> {
    /// Per-instance sequence number, monotonically increasing.
    /// Establishes total ordering of transitions within a single FSM instance.
    pub seq: u64,
    /// The state being entered and its attributes.
    pub state: S,
}

// An FSM's event stream is named after its transition set. `FsmEvent` is foreign
// to the crate that defines an FSM, so its transition enum (which is local
// there) carries the name and this blanket forwards it.
impl<S> quent_events::EntityEvent for FsmEvent<S>
where
    S: quent_events::EntityEvent,
{
    const NAME: &'static str = S::NAME;
}
