// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared helpers for constraint implementations.

use std::fmt::Display;

use quent_schema::{DataType, Path, Record};
use thiserror::Error;

/// Format items as a bulleted, newline-separated list.
pub fn bullet_list<T: Display>(items: &[T]) -> String {
    items
        .iter()
        .map(|item| format!("  - {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Validates a record against the required fields of an expected record.
pub struct RecordValidator<'e> {
    expected: &'e Record,
}

impl<'e> RecordValidator<'e> {
    /// Create a validator for the fields declared by `expected`.
    pub fn new(expected: &'e Record) -> Self {
        Self { expected }
    }

    /// Validate `actual`, returning every missing or mistyped required field.
    ///
    /// Fields not declared by the expected record are permitted.
    pub fn validate(&self, actual: &Record) -> Vec<RecordValidationError> {
        let mut errors = Vec::new();
        for expected_field in self.expected.fields() {
            let field = expected_field.name();
            match actual.field(field) {
                None => errors.push(RecordValidationError::MissingField {
                    record: actual.path().clone(),
                    field: field.to_string(),
                }),
                Some(actual_field) if actual_field.ty() != expected_field.ty() => {
                    errors.push(RecordValidationError::InvalidFieldType {
                        record: actual.path().clone(),
                        field: field.to_string(),
                        expected: Box::new(expected_field.ty().clone()),
                        actual: Box::new(actual_field.ty().clone()),
                    });
                }
                Some(_) => {}
            }
        }
        errors
    }
}

/// A record field failed a [`RecordValidator`] requirement.
#[derive(Debug, Error)]
pub enum RecordValidationError {
    #[error("{record}: record is missing field `{field}`")]
    MissingField { record: Path, field: String },
    #[error("{record}.{field}: expected {expected:?}, found {actual:?}")]
    InvalidFieldType {
        record: Path,
        field: String,
        expected: Box<DataType>,
        actual: Box<DataType>,
    },
}

#[cfg(test)]
mod tests {
    use quent_schema::{
        DataType,
        builder::RecordBuilder,
        test_utils::{field, path},
    };

    use super::{RecordValidationError, RecordValidator};

    #[test]
    fn aggregates_missing_and_invalid_fields() {
        let expected = RecordBuilder::new(path("Expected"))
            .with_fields([field("id", DataType::I32), field("process", DataType::Uuid)])
            .build()
            .unwrap();
        let actual = RecordBuilder::new(path("Actual"))
            .with_fields([field("id", DataType::U32), field("extra", DataType::String)])
            .build()
            .unwrap();

        let errors = RecordValidator::new(&expected).validate(&actual);

        assert!(matches!(
            errors[0],
            RecordValidationError::InvalidFieldType { .. }
        ));
        assert!(matches!(
            errors[1],
            RecordValidationError::MissingField { .. }
        ));
    }
}
