// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generic entity event storage reconstructed from model-generated events.

use quent_events::Event;
use quent_model::EntityData;
use quent_time::TimeUnixNanoSec;
use uuid::Uuid;

use crate::{AnalyzerError, AnalyzerResult};

/// Event storage for a model-defined entity.
///
/// `M` is the model marker type (e.g., `engine::Engine`). The data struct
/// `M::Data` stores one `Option<T>` per event type, populated by `push()`.
///
/// ```ignore
/// let engine: EntityEvents<engine::Engine> = ...;
/// let name = engine.data().init.as_ref().unwrap().instance_name.clone();
/// ```
pub struct EntityEvents<M: EntityData> {
    id: Uuid,
    earliest_timestamp: Option<TimeUnixNanoSec>,
    latest_timestamp: Option<TimeUnixNanoSec>,
    data: M::Data,
}

impl<M: EntityData> std::fmt::Debug for EntityEvents<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityEvents")
            .field("id", &self.id)
            .field("earliest_timestamp", &self.earliest_timestamp)
            .field("latest_timestamp", &self.latest_timestamp)
            .finish_non_exhaustive()
    }
}

impl<M: EntityData> EntityEvents<M> {
    pub fn new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "entity id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                earliest_timestamp: None,
                latest_timestamp: None,
                data: M::Data::default(),
            })
        }
    }

    pub fn push(&mut self, event: Event<M::Event>) {
        let ts = event.timestamp;
        self.earliest_timestamp = Some(match self.earliest_timestamp {
            Some(prev) => prev.min(ts),
            None => ts,
        });
        self.latest_timestamp = Some(match self.latest_timestamp {
            Some(prev) => prev.max(ts),
            None => ts,
        });
        M::push(&mut self.data, event.data);
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn data(&self) -> &M::Data {
        &self.data
    }

    pub fn earliest_timestamp(&self) -> Option<TimeUnixNanoSec> {
        self.earliest_timestamp
    }

    pub fn latest_timestamp(&self) -> Option<TimeUnixNanoSec> {
        self.latest_timestamp
    }
}
