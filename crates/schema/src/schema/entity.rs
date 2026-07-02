// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::schema::{Map, annotations::Annotations, event::Event, identifier::Identifier};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Entity {
    /// The name of the entity.
    name: Identifier,
    /// The events that this entity can emit.
    #[cfg_attr(feature = "ts", ts(as = "indexmap::IndexMap<Identifier, Event>"))]
    events: Map<Identifier, Event>,
    /// Annotations of this entity.
    annotations: Annotations,
}

impl Entity {
    pub(crate) fn from_parts(
        name: Identifier,
        events: Map<Identifier, Event>,
        annotations: Annotations,
    ) -> Self {
        Self {
            name,
            events,
            annotations,
        }
    }

    /// The name of the entity.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// The annotations of this entity.
    pub fn annotations(&self) -> &Annotations {
        &self.annotations
    }

    /// The event declared under `name`, if any.
    pub fn event(&self, name: &Identifier) -> Option<&Event> {
        self.events.get(name)
    }

    /// The declared events, in declaration order.
    pub fn events(&self) -> impl Iterator<Item = &Event> + '_ {
        self.events.values()
    }
}
