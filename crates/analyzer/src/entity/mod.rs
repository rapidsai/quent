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
    first_timestamp: Option<TimeUnixNanoSec>,
    last_timestamp: Option<TimeUnixNanoSec>,
    data: M::Data,
}

impl<M: EntityData> std::fmt::Debug for EntityEvents<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityEvents")
            .field("id", &self.id)
            .field("first_timestamp", &self.first_timestamp)
            .field("last_timestamp", &self.last_timestamp)
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
                first_timestamp: None,
                last_timestamp: None,
                data: M::Data::default(),
            })
        }
    }

    pub fn push(&mut self, event: Event<M::Event>) {
        if self.first_timestamp.is_none() {
            self.first_timestamp = Some(event.timestamp);
        }
        self.last_timestamp = Some(event.timestamp);
        M::push(&mut self.data, event.data);
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn data(&self) -> &M::Data {
        &self.data
    }

    pub fn first_timestamp(&self) -> Option<TimeUnixNanoSec> {
        self.first_timestamp
    }

    pub fn last_timestamp(&self) -> Option<TimeUnixNanoSec> {
        self.last_timestamp
    }
}
