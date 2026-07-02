// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::schema::{Map, constraint::Constraint, metadata::Metadata};

/// Annotations for [`crate::Schema`] constituents.
///
/// Annotations do not affect how events are read or written. They merely carry
/// documentation about core elements of the Schema, opaque [`Constraint`]s that
/// must be validated if they cannot be guaranteed to hold, and miscellaneous
/// opaque [`Metadata`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Annotations {
    /// Potential documentation that can e.g. be added in code generation.
    docs: Option<String>,
    /// Opaque constraints that must be validated against the schema.
    #[cfg_attr(feature = "ts", ts(as = "indexmap::IndexMap<String, Constraint>"))]
    constraints: Map<String, Constraint>,
    /// Opaque metadata passed through the schema.
    #[cfg_attr(feature = "ts", ts(as = "indexmap::IndexMap<String, Metadata>"))]
    metadata: Map<String, Metadata>,
}

impl Annotations {
    pub(crate) fn from_parts(
        docs: Option<String>,
        constraints: Map<String, Constraint>,
        metadata: Map<String, Metadata>,
    ) -> Self {
        Self {
            docs,
            constraints,
            metadata,
        }
    }

    /// The documentation, if any.
    pub fn docs(&self) -> Option<&str> {
        self.docs.as_deref()
    }

    /// The constraint declared under `name`, if any.
    pub fn constraint(&self, name: &str) -> Option<&Constraint> {
        self.constraints.get(name)
    }

    /// Return true if the constraint declared under `name` is set.
    pub fn has_constraint(&self, name: &str) -> bool {
        self.constraint(name).is_some()
    }

    /// The declared constraints, in declaration order.
    pub fn constraints(&self) -> impl Iterator<Item = &Constraint> + '_ {
        self.constraints.values()
    }

    /// The metadata declared under `name`, if any.
    pub fn metadata(&self, name: &str) -> Option<&Metadata> {
        self.metadata.get(name)
    }

    /// The declared metadata, in declaration order.
    pub fn metadata_entries(&self) -> impl Iterator<Item = &Metadata> + '_ {
        self.metadata.values()
    }
}
