// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::builder::{AnnotationsBuilder, BuilderError, collect_unique};
use crate::{Annotations, Field, Path, PathError, Record};

/// Builder for a [`Record`].
pub struct RecordBuilder {
    path: Path,
    fields: Vec<Field>,
    annotations: AnnotationsBuilder,
}

impl RecordBuilder {
    /// Starts a record at the supplied qualified path.
    pub fn new(path: impl Into<Path>) -> Self {
        Self {
            path: path.into(),
            fields: Vec::new(),
            annotations: AnnotationsBuilder::new(),
        }
    }

    /// Start a record at `path`, validating its segments.
    ///
    /// # Errors
    ///
    /// Errors if `path` is not a valid path.
    pub fn try_new(path: impl AsRef<str>) -> Result<Self, PathError> {
        Ok(Self::new(path.as_ref().parse::<Path>()?))
    }

    /// Add a field, returning the builder for chaining.
    pub fn with_field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    /// Add several fields, returning the builder for chaining.
    pub fn with_fields(mut self, fields: impl IntoIterator<Item = Field>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Set the record's annotations, replacing any added so far, and return
    /// the builder for chaining.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Finish building the record.
    ///
    /// # Errors
    ///
    /// Errors if a field name is repeated or the annotations are invalid.
    pub fn build(self) -> Result<Record, BuilderError> {
        let Self {
            path,
            fields,
            annotations,
        } = self;
        let fields = collect_unique(fields, |field| field.name().clone())?;
        let annotations = annotations.build()?;
        Ok(Record::from_parts(path, fields, annotations))
    }
}
