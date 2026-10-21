// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Canonical NVTX records and the NVTX entity.

use quent_constraints::{
    Constraint,
    utils::{RecordValidationError, RecordValidator, bullet_list},
};
use quent_schema::{
    Cardinality, DataType, Entity, Path, Record,
    visitor::{Cursor, Element, Visitor},
};
use thiserror::Error;

mod builder;
mod definitions;

pub use builder::NvtxEntityBuilder;
pub use definitions::canonical_records;
use definitions::{EVENTS, ident, path};

/// Returns the canonical NVTX entity path.
pub fn nvtx_path() -> Path {
    path("Sink")
}

/// Validates canonical NVTX records and the fixed `quent::nvtx::Sink` entity contract.
///
/// Callers must also check base validation.
///
/// ## Requirements
///
/// 1. The canonical entity has exactly one `Once` event named `initialized`, whose
///    sole `process` field is an ordinary, type-erased reference without data.
///    At runtime, callers are responsible for referencing a process entity if
///    they want to correlate NVTX events with a process. The reference target is
///    not validated.
/// 2. The entity has all 15 repeatable NVTX events carrying canonical records in
///    their sole `data` field.
/// 3. Canonical records have exactly the declared fields and carry this constraint.
/// 4. The annotation has no payload and is valid only on the canonical entity and
///    records.
pub struct NvtxConstraint {
    errors: Vec<NvtxError>,
    records: Vec<Record>,
}

impl Default for NvtxConstraint {
    fn default() -> Self {
        Self {
            errors: Vec::new(),
            records: canonical_records(),
        }
    }
}

impl Constraint for NvtxConstraint {
    const NAME: &'static str = "quent.nvtx.v0.1.0";
}

impl Visitor for NvtxConstraint {
    type Output = Result<(), NvtxError>;

    fn visit(&mut self, cursor: &Cursor) {
        match cursor.current() {
            Element::Record(record) => {
                if let Some(expected) = self
                    .records
                    .iter()
                    .find(|expected| expected.path() == record.path())
                {
                    if !record.annotations().has_constraint(Self::NAME) {
                        self.errors.push(NvtxError::MissingAnnotation {
                            record: record.path().clone(),
                        });
                    }
                    self.errors.extend(
                        RecordValidator::new(expected)
                            .validate_exact(record)
                            .into_iter()
                            .map(NvtxError::InvalidRecord),
                    );
                }
            }
            Element::Entity(entity)
                if entity.path() == &nvtx_path()
                    || entity.annotations().has_constraint(Self::NAME) =>
            {
                self.check_sink(entity);
            }
            Element::Annotations(annotations) if annotations.has_constraint(Self::NAME) => {
                let valid_location = match cursor.previous() {
                    Some(Element::Entity(_)) => true,
                    Some(Element::Record(record)) => self
                        .records
                        .iter()
                        .any(|expected| expected.path() == record.path()),
                    _ => false,
                };
                if !valid_location {
                    self.errors.push(NvtxError::Misplaced {
                        location: cursor.to_string(),
                    });
                }
                if annotations
                    .constraint(Self::NAME)
                    .and_then(|constraint| constraint.data())
                    .is_some()
                {
                    self.errors.push(NvtxError::UnexpectedData {
                        location: cursor.to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    fn finish(self) -> Self::Output {
        match self.errors.len() {
            0 => Ok(()),
            1 => Err(self.errors.into_iter().next().unwrap()),
            _ => Err(NvtxError::Multiple(self.errors)),
        }
    }
}

impl NvtxConstraint {
    fn invalid_sink(&mut self, entity: &Entity, message: impl Into<String>) {
        self.errors.push(NvtxError::InvalidSink {
            entity: entity.path().clone(),
            message: message.into(),
        });
    }

    fn check_sink(&mut self, entity: &Entity) {
        if entity.path() != &nvtx_path() {
            self.invalid_sink(entity, "NVTX entity must be named `quent::nvtx::Sink`");
        }
        if !entity.annotations().has_constraint(Self::NAME) {
            self.invalid_sink(
                entity,
                format!("NVTX entity must carry constraint `{}`", Self::NAME),
            );
        }
        self.check_initialization(entity);
        for (name, record) in EVENTS {
            let Some(event) = entity.event(&ident(name)) else {
                self.invalid_sink(entity, format!("missing event `{name}`"));
                continue;
            };
            if event.cardinality() != Cardinality::Multi {
                self.invalid_sink(entity, format!("event `{name}` must be repeatable"));
            }
            if event.fields().count() != 1
                || event.field(&ident("data")).map(|field| field.ty())
                    != Some(&DataType::Record(path(record)))
            {
                self.invalid_sink(
                    entity,
                    format!("event `{name}` must contain exactly `data: quent::nvtx::{record}`"),
                );
            }
        }
        for event in entity.events() {
            if event.name() != &ident("initialized")
                && !EVENTS.iter().any(|(name, _)| event.name() == &ident(name))
            {
                self.invalid_sink(entity, format!("unexpected event `{}`", event.name()));
            }
        }
    }

    fn check_initialization(&mut self, entity: &Entity) {
        let Some(event) = entity.event(&ident("initialized")) else {
            self.invalid_sink(entity, "missing event `initialized`");
            return;
        };
        if event.cardinality() != Cardinality::Once {
            self.invalid_sink(entity, "event `initialized` must have cardinality `Once`");
        }
        let reference = event.field(&ident("process")).map(|field| field.ty());
        let Some(DataType::EntityRef {
            data: None,
            annotations,
        }) = reference
        else {
            self.invalid_sink(entity, "event `initialized` must contain exactly one nonoptional reference `process` without data");
            return;
        };
        if event.fields().count() != 1 {
            self.invalid_sink(entity, "event `initialized` must contain only `process`");
        }
        if annotations.constraints().next().is_some() {
            self.invalid_sink(entity, "`initialized.process` must be an ordinary type-erased reference without constraints");
        }
    }
}

/// A violation of the canonical NVTX schema contract.
#[derive(Debug, Error)]
pub enum NvtxError {
    #[error(
        "canonical NVTX record `{record}` must carry constraint `{}`",
        NvtxConstraint::NAME
    )]
    MissingAnnotation { record: Path },
    #[error(transparent)]
    InvalidRecord(#[from] RecordValidationError),
    #[error("NVTX sink `{entity}`: {message}")]
    InvalidSink { entity: Path, message: String },
    #[error(
        "{location}: the NVTX constraint is valid only on the canonical NVTX entity and canonical NVTX records"
    )]
    Misplaced { location: String },
    #[error("{location}: the NVTX constraint must not carry data")]
    UnexpectedData { location: String },
    #[error("multiple NVTX constraint violations:\n{}", bullet_list(.0))]
    Multiple(Vec<NvtxError>),
}
