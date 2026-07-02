// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::schema::{Map, annotations::Annotations, field::Field, identifier::Identifier};

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum Cardinality {
    /// The event can be emitted zero or one time.
    Once,
    /// The event can be emitted zero or multiple times.
    Multi,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Event {
    /// The name of the event.
    name: Identifier,
    /// The [`Cardinality`] of the event.
    cardinality: Cardinality,
    /// The payload fields of the event.
    #[cfg_attr(feature = "ts", ts(as = "indexmap::IndexMap<Identifier, Field>"))]
    payload: Map<Identifier, Field>,
    /// Annotations of this event.
    annotations: Annotations,
}

impl Event {
    pub(crate) fn from_parts(
        name: Identifier,
        cardinality: Cardinality,
        payload: Map<Identifier, Field>,
        annotations: Annotations,
    ) -> Self {
        Self {
            name,
            cardinality,
            payload,
            annotations,
        }
    }

    /// The name of the event.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// The [`Cardinality`] of the event.
    pub fn cardinality(&self) -> Cardinality {
        self.cardinality
    }

    /// The annotations of this event.
    pub fn annotations(&self) -> &Annotations {
        &self.annotations
    }

    /// The payload field declared under `name`, if any.
    pub fn field(&self, name: &Identifier) -> Option<&Field> {
        self.payload.get(name)
    }

    /// The payload fields, in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = &Field> + '_ {
        self.payload.values()
    }
}
