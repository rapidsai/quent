// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint as _;
use quent_schema::{
    Annotations, Cardinality, DataType, Entity,
    builder::{AnnotationsBuilder, BuilderError, EntityBuilder, EventBuilder},
};

use crate::{
    NvtxConstraint,
    definitions::{EVENTS, field, ident, path},
    nvtx_path,
};

/// Builds a fixed NVTX sink with a once-only process reference and repeatable events.
#[derive(Default)]
pub struct NvtxEntityBuilder {
    annotations: Annotations,
}

impl NvtxEntityBuilder {
    /// Starts the canonical NVTX entity.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set annotations, preserving them alongside the NVTX constraint.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = annotations;
        self
    }

    /// Builds the sink entity without validating its referenced records.
    ///
    /// Validate the completed schema with [`NvtxConstraint`] before using the entity.
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied annotations are invalid or conflict
    /// with the generated NVTX constraint.
    pub fn build(self) -> Result<Entity, BuilderError> {
        let initialized = EventBuilder::new(ident("initialized"), Cardinality::Once)
            .with_field(field(
                "process",
                DataType::EntityRef {
                    data: None,
                    annotations: Annotations::default(),
                },
            ))
            .build()?;
        let annotations = AnnotationsBuilder::from_annotations(&self.annotations)
            .with_constraint(NvtxConstraint::NAME, None)
            .build()?;
        let mut entity = EntityBuilder::new(nvtx_path())
            .with_annotations(annotations)
            .with_event(initialized);
        for (name, record) in EVENTS {
            entity = entity.with_event(
                EventBuilder::new(ident(name), Cardinality::Multi)
                    .with_field(field("data", DataType::Record(path(record))))
                    .build()?,
            );
        }
        entity.build()
    }
}
