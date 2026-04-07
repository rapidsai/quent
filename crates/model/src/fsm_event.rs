// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Common event wrapper for all FSM event types.

/// Common wrapper for all FSM events.
///
/// Every generated FSM event type is an alias over `FsmEvent<S, D>`:
///
/// ```text
/// pub type TaskEvent = FsmEvent<TaskTransition, TaskDeferred>;
/// ```
///
/// `S` is the transition enum (one variant per state + exit).
/// `D` is the deferred enum (one variant per deferred field across all states).
///
/// For FSMs with no deferred fields, `D` is an empty (uninhabitable) enum.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FsmEvent<S, D> {
    /// A transition into a new state.
    Transition {
        /// Per-instance sequence number, monotonically increasing.
        seq: u64,
        /// The state being entered and its attributes.
        state: S,
    },
    /// A deferred field update for the current state.
    Deferred {
        /// Per-instance sequence number, monotonically increasing.
        seq: u64,
        /// The deferred field being set.
        deferred: D,
    },
}

impl<S, D> FsmEvent<S, D> {
    /// Returns the sequence number of this event.
    pub fn seq(&self) -> u64 {
        match self {
            FsmEvent::Transition { seq, .. } => *seq,
            FsmEvent::Deferred { seq, .. } => *seq,
        }
    }
}
