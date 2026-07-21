// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::builder::{BuilderError, insert_unique};
use crate::schema::Map;
use crate::{Annotations, Constraint, Metadata};

/// Builder for a map of named items.
struct OpaqueMapBuilder<T>(Map<String, T>);

impl<T> Default for OpaqueMapBuilder<T> {
    fn default() -> Self {
        Self(Map::default())
    }
}

impl<T> OpaqueMapBuilder<T> {
    fn try_insert(&mut self, name: String, value: T) -> Result<(), BuilderError> {
        if name.is_empty() {
            return Err(BuilderError::EmptyName);
        }
        insert_unique(&mut self.0, name, value)
    }

    fn set(&mut self, name: String, value: T) -> Option<T> {
        self.0.insert(name, value)
    }
}

/// Builder for [`Annotations`].
#[derive(Default)]
pub struct AnnotationsBuilder {
    docs: Option<String>,
    constraints: OpaqueMapBuilder<Constraint>,
    metadata: OpaqueMapBuilder<Metadata>,
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
            constraints: OpaqueMapBuilder(
                annotations
                    .constraints()
                    .map(|c| (c.name().to_owned(), c.clone()))
                    .collect(),
            ),
            metadata: OpaqueMapBuilder(
                annotations
                    .metadata_entries()
                    .map(|m| (m.name().to_owned(), m.clone()))
                    .collect(),
            ),
        }
    }

    /// The documentation, if set.
    pub fn docs(&self) -> Option<&str> {
        self.docs.as_deref()
    }

    /// Set the documentation string, returning the replaced one, if any.
    pub fn set_docs(&mut self, docs: impl Into<String>) -> Option<String> {
        self.docs.replace(docs.into())
    }

    /// The constraint declared under `name`, if any.
    pub fn constraint(&self, name: &str) -> Option<&Constraint> {
        self.constraints.0.get(name)
    }

    /// Add a constraint named `name`.
    ///
    /// # Errors
    ///
    /// Errors if `name` is empty or already declared.
    pub fn try_insert_constraint(
        &mut self,
        name: impl Into<String>,
        data: Option<String>,
    ) -> Result<&mut Self, BuilderError> {
        let name = name.into();
        self.constraints
            .try_insert(name.clone(), Constraint::from_parts(name, data))?;
        Ok(self)
    }

    /// Add a constraint named `name`, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors if `name` is empty or already declared.
    pub fn try_with_constraint(
        mut self,
        name: impl Into<String>,
        data: Option<String>,
    ) -> Result<Self, BuilderError> {
        self.try_insert_constraint(name, data)?;
        Ok(self)
    }

    /// Set the constraint named `name`, returning the replaced entry, if any.
    pub fn set_constraint(
        &mut self,
        name: impl Into<String>,
        data: Option<String>,
    ) -> Option<Constraint> {
        let name = name.into();
        self.constraints
            .set(name.clone(), Constraint::from_parts(name, data))
    }

    /// The metadata entry declared under `name`, if any.
    pub fn metadata(&self, name: &str) -> Option<&Metadata> {
        self.metadata.0.get(name)
    }

    /// Add a metadata entry named `name`.
    ///
    /// # Errors
    ///
    /// Errors if `name` is empty or already declared.
    pub fn try_insert_metadata(
        &mut self,
        name: impl Into<String>,
        data: Option<String>,
    ) -> Result<&mut Self, BuilderError> {
        let name = name.into();
        self.metadata
            .try_insert(name.clone(), Metadata::from_parts(name, data))?;
        Ok(self)
    }

    /// Add a metadata entry named `name`, returning the builder for chaining.
    ///
    /// # Errors
    ///
    /// Errors if `name` is empty or already declared.
    pub fn try_with_metadata(
        mut self,
        name: impl Into<String>,
        data: Option<String>,
    ) -> Result<Self, BuilderError> {
        self.try_insert_metadata(name, data)?;
        Ok(self)
    }

    /// Set the metadata entry named `name`, returning the replaced entry, if any.
    pub fn set_metadata(
        &mut self,
        name: impl Into<String>,
        data: Option<String>,
    ) -> Option<Metadata> {
        let name = name.into();
        self.metadata
            .set(name.clone(), Metadata::from_parts(name, data))
    }

    /// Finish building the annotations.
    pub fn build(self) -> Annotations {
        Annotations::from_parts(self.docs, self.constraints.0, self.metadata.0)
    }
}
