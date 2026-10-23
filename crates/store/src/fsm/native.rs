// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Rust-native FSM handle implementation support.

use crate::{
    Timestamped,
    entity::native::{NativeEntity, NativeEntityMarker},
    fsm::{FsmHandle, Transition},
};

/// Associates an FSM entity marker with borrowing transition handles.
///
/// Collected events must be ordered by timestamp and then recorded transition sequence number.
#[doc(hidden)]
pub trait NativeFsmMarker: NativeEntityMarker {
    /// A handle borrowing one transition event.
    type Transition<'a>: Transition
    where
        Self: 'a;

    /// Borrows the transition represented by `event`.
    fn transition(event: &Timestamped<Self::Payload>) -> Self::Transition<'_>;
}

impl<E: NativeFsmMarker> FsmHandle for NativeEntity<E> {
    type Transition<'a>
        = E::Transition<'a>
    where
        Self: 'a;

    fn transition(&self, index: usize) -> Option<Self::Transition<'_>> {
        self.event_sequence().events().get(index).map(E::transition)
    }
}
