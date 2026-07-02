// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/// Opaque named constraint.
///
/// Used to impose rules on a model that must hold for the schema to be
/// logically sound, and therefore requires validation against the entire
/// schema. The canonical validation mechanism is provided by the
/// `quent-constraints` crate, which matches a constraint to its validator by
/// [`Constraint::name`].
///
/// For opaque data that carries no validation requirement, see
/// [`crate::metadata::Metadata`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Constraint {
    /// The name of the constraint.
    name: String,
    /// Constraint-specific opaque data.
    ///
    /// The format for this data is unrestricted, other than that it must form a
    /// valid UTF-8 string. It is encouraged to serialize Constraint data in a
    /// human-readable fashion for easier debugging.
    data: Option<String>,
}

impl Constraint {
    pub(crate) fn from_parts(name: String, data: Option<String>) -> Self {
        Self { name, data }
    }

    /// The name of the constraint.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The constraint-specific opaque data, if any.
    pub fn data(&self) -> Option<&str> {
        self.data.as_deref()
    }
}
