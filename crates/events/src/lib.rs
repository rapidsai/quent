// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Type definitions of entity events.

use quent_time::{TimeUnixNanoSec, Timestamp, timestamp};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trait for the event type of an entity.
pub trait EntityEvent {
    /// The name of the entity.
    const NAME: &'static str;
}

#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Debug)]
pub struct Event<T> {
    /// The ID of the entity producing this event.
    pub id: Uuid,
    /// The timestamp of the event.
    pub timestamp: TimeUnixNanoSec,
    /// The payload of the event.
    pub data: T,
}

impl<T> Event<T> {
    #[inline(always)]
    pub fn new_now(id: Uuid, data: T) -> Self {
        Self {
            id,
            timestamp: timestamp(),
            data,
        }
    }

    #[inline(always)]
    pub fn new(id: Uuid, timestamp: TimeUnixNanoSec, data: T) -> Self {
        Self {
            id,
            timestamp,
            data,
        }
    }
}

impl<T> Timestamp for Event<T> {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}
