// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Plain schema data types

use crate::Path;
use crate::schema::{
    annotations::Annotations, entity::Entity, identifier::Identifier, record::Record,
};

pub mod annotations;
pub mod constraint;
pub mod data_type;
pub mod entity;
pub mod event;
pub mod field;
pub mod identifier;
pub mod metadata;
pub mod path;
pub mod record;

/// Container type for named elements.
pub(crate) type Map<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    /// The name of the model.
    name: Identifier,
    /// The [`Entity`]s of the model.
    #[cfg_attr(feature = "serde", serde(with = "indexmap::map::serde_seq"))]
    #[cfg_attr(feature = "ts", ts(as = "Vec<(Path, Entity)>"))]
    entities: Map<Path, Entity>,
    /// The [`Record`]s of the model.
    #[cfg_attr(feature = "serde", serde(with = "indexmap::map::serde_seq"))]
    #[cfg_attr(feature = "ts", ts(as = "Vec<(Path, Record)>"))]
    records: Map<Path, Record>,
    /// Annotations of this schema.
    annotations: Annotations,
}

impl Schema {
    pub(crate) fn from_parts(
        name: Identifier,
        entities: Map<Path, Entity>,
        records: Map<Path, Record>,
        annotations: Annotations,
    ) -> Self {
        Self {
            name,
            entities,
            records,
            annotations,
        }
    }

    /// The name of the model.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// The annotations of this schema.
    pub fn annotations(&self) -> &Annotations {
        &self.annotations
    }

    /// Returns the entity declared at `path`, if any.
    pub fn entity(&self, path: &Path) -> Option<&Entity> {
        self.entities.get(path)
    }

    /// The declared entities, in declaration order.
    pub fn entities(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.entities.values()
    }

    /// Returns the record declared at `path`, if any.
    pub fn record(&self, path: &Path) -> Option<&Record> {
        self.records.get(path)
    }

    /// The declared records, in declaration order.
    pub fn records(&self) -> impl Iterator<Item = &Record> + '_ {
        self.records.values()
    }
}
