// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::builder::BuilderError;
use crate::schema::Map;
use crate::{Annotations, Constraint, Metadata};

/// Builder for [`Annotations`].
#[derive(Default)]
pub struct AnnotationsBuilder {
    docs: Option<String>,
    constraints: Vec<Constraint>,
    metadata: Vec<Metadata>,
}

impl AnnotationsBuilder {
    /// Start with empty annotations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start from existing annotations.
    pub fn from_annotations(annotations: &Annotations) -> Self {
        Self {
            docs: annotations.docs().map(str::to_owned),
            constraints: annotations.constraints().cloned().collect(),
            metadata: annotations.metadata_entries().cloned().collect(),
        }
    }

    /// Set the documentation string.
    pub fn with_docs(mut self, docs: impl Into<String>) -> Self {
        self.docs = Some(docs.into());
        self
    }

    /// Add a constraint named `name`, returning the builder for chaining.
    pub fn with_constraint(mut self, name: impl Into<String>, data: Option<String>) -> Self {
        let name = name.into();
        self.constraints.push(Constraint::from_parts(name, data));
        self
    }

    /// Add a metadata entry named `name`, returning the builder for chaining.
    pub fn with_metadata(mut self, name: impl Into<String>, data: Option<String>) -> Self {
        let name = name.into();
        self.metadata.push(Metadata::from_parts(name, data));
        self
    }

    /// Finish building the annotations.
    ///
    /// # Errors
    ///
    /// Errors if a name is empty or repeated.
    pub fn build(self) -> Result<Annotations, BuilderError> {
        let Self {
            docs,
            constraints,
            metadata,
        } = self;
        let constraints = collect_named(constraints, Constraint::name)?;
        let metadata = collect_named(metadata, Metadata::name)?;
        Ok(Annotations::from_parts(docs, constraints, metadata))
    }
}

fn collect_named<T>(
    values: impl IntoIterator<Item = T>,
    mut name: impl FnMut(&T) -> &str,
) -> Result<Map<String, T>, BuilderError> {
    let mut map = Map::default();
    for value in values {
        let name = name(&value).to_owned();
        if name.is_empty() {
            return Err(BuilderError::EmptyName);
        }
        match map.entry(name) {
            indexmap::map::Entry::Occupied(entry) => {
                return Err(BuilderError::DuplicateName(entry.key().clone()));
            }
            indexmap::map::Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }
    }
    Ok(map)
}
