// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::Path;
use crate::schema::{Map, annotations::Annotations, event::Event, identifier::Identifier};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Entity {
    /// The path of the entity.
    path: Path,
    /// The events that this entity can emit.
    #[cfg_attr(feature = "ts", ts(as = "indexmap::IndexMap<Identifier, Event>"))]
    events: Map<Identifier, Event>,
    /// Annotations of this entity.
    annotations: Annotations,
}

impl Entity {
    pub(crate) fn from_parts(
        path: Path,
        events: Map<Identifier, Event>,
        annotations: Annotations,
    ) -> Self {
        Self {
            path,
            events,
            annotations,
        }
    }

    /// The path of the entity.
    pub fn path(&self) -> &Path {
        &self.path
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
    pub fn events(&self) -> impl ExactSizeIterator<Item = &Event> + '_ {
        self.events.values()
    }
}
