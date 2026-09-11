// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The Quent built-in log-sink constraint.
//!
//! A log sink is an entity whose events represent severity-ranked messages.
//! The entity path supplies the static scope, while each entity instance
//! supplies runtime identity. Every declared level is represented by a
//! repeatable event with a required `message: string` field. Optional standard
//! fields preserve facade-provided target and source information.
//! Level names are copied verbatim to event names and must follow the
//! [`Identifier`] grammar `[A-Za-z][A-Za-z0-9_]*`. No scope or runtime level
//! attribute is generated; the entity and event identities provide them.
//!
//! ## YAML
//!
//! A minimal sink declares only its ordered levels:
//!
//! ```yaml
//! entities:
//!   AppLog:
//!     log:
//!       levels:
//!         - name: trace
//!         - name: debug
//!         - name: info
//!         - name: warning
//!         - name: error
//! ```
//!
//! Common and level-specific attributes use the ordinary YAML field syntax:
//!
//! ```yaml
//! entities:
//!   AppLog:
//!     doc: Application logging sink.
//!     log:
//!       target: true
//!       source: true
//!       attributes:
//!         thread_name: { option: string }
//!       levels:
//!         - name: info
//!           doc: Informational messages.
//!         - name: error
//!           attributes:
//!             error_code: { option: u32 }
//! ```
//!
//! Instrumentation generation treats these as ordinary repeatable events. For
//! example, the complete form produces `AppLogEvent::Info` and an `AppLog`
//! handle method accepting `message`, the enabled standard fields, and
//! `thread_name`. Adapters for logging facades can inspect [`LogDefinition`]
//! to discover that schema contract; this crate does not provide an adapter.
//!
//! ## Requirements
//!
//! For every entity carrying this constraint:
//!
//! 1. The ordered level list contains between one and 256 unique valid
//!    [`Identifier`] values. A level's zero-based position is its `u8` rank.
//! 2. Every level names an event, and every event is named by a level.
//! 3. Every level event has [`Cardinality::Multi`].
//! 4. Every level event contains `message: string`.
//! 5. When enabled, every level event contains `target: option<string>`.
//! 6. Enabled source fields occur on every level event as `file:
//!    option<string>`, `line: option<u32>`, and `module: option<string>`.
//! 7. Additional event fields are allowed.
//! 8. The constraint appears only on the log-sink entity.

use std::collections::HashSet;

use quent_constraints::{Constraint, utils::bullet_list};
use quent_schema::{
    Cardinality, DataType, Entity, Identifier, Path,
    visitor::{Cursor, Element, Visitor},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod builder;

pub use builder::{LevelDecl, LogEntityBuilder, LogEntityBuilderError};

/// Maximum number of levels in one log definition.
pub const MAX_LEVELS: usize = u8::MAX as usize + 1;

/// Standard source attributes supported by a log sink.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFields {
    file: bool,
    line: bool,
    module: bool,
}

impl SourceFields {
    /// Enable all supported source attributes.
    pub const fn all() -> Self {
        Self {
            file: true,
            line: true,
            module: true,
        }
    }

    /// Select individual source attributes.
    pub const fn new(file: bool, line: bool, module: bool) -> Self {
        Self { file, line, module }
    }

    /// Whether the optional `file` attribute is enabled.
    pub const fn file(self) -> bool {
        self.file
    }

    /// Whether the optional `line` attribute is enabled.
    pub const fn line(self) -> bool {
        self.line
    }

    /// Whether the optional `module` attribute is enabled.
    pub const fn module(self) -> bool {
        self.module
    }
}

/// Definition carried by a log-sink entity's constraint annotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogDefinition {
    levels: Vec<Identifier>,
    target: bool,
    source: SourceFields,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLogDefinition {
    levels: Vec<String>,
    #[serde(default)]
    target: bool,
    #[serde(default)]
    source: SourceFields,
}

impl LogDefinition {
    /// Constraint identifier.
    pub const NAME: &'static str = "quent.log.v0.1.0";

    /// Create a validated log definition.
    ///
    /// # Errors
    ///
    /// Returns an error if levels are empty, repeated, or exceed 256 entries.
    pub fn new(
        levels: Vec<Identifier>,
        target: bool,
        source: SourceFields,
    ) -> Result<Self, LogDefinitionError> {
        validate_levels(&levels)?;
        Ok(Self {
            levels,
            target,
            source,
        })
    }

    /// Decode the log definition on `entity`, or return `None` if it is not a
    /// log sink.
    pub fn from_entity(entity: &Entity) -> Option<Result<Self, LogDefinitionError>> {
        let constraint = entity.annotations().constraint(Self::NAME)?;
        Some(Self::decode(constraint.data()))
    }

    /// Return levels in rank order.
    pub fn levels(&self) -> impl ExactSizeIterator<Item = RankedLevel<'_>> {
        self.levels
            .iter()
            .enumerate()
            .map(|(rank, name)| RankedLevel {
                name,
                rank: rank as u8,
            })
    }

    /// Return the ranked level named `name`, if declared.
    pub fn level(&self, name: &Identifier) -> Option<RankedLevel<'_>> {
        self.levels().find(|level| level.name == name)
    }

    /// Whether the optional `target` attribute is enabled.
    pub const fn target_enabled(&self) -> bool {
        self.target
    }

    /// Return the enabled source attributes.
    pub const fn source(&self) -> SourceFields {
        self.source
    }

    /// Encode this definition as a constraint payload.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn constraint_data(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    fn decode(data: Option<&str>) -> Result<Self, LogDefinitionError> {
        let data = data.ok_or(LogDefinitionError::MissingData)?;
        let raw: RawLogDefinition =
            serde_json::from_str(data).map_err(|error| LogDefinitionError::MalformedData {
                message: error.to_string(),
            })?;
        let levels = raw
            .levels
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                Identifier::try_new(name.clone()).map_err(|error| {
                    LogDefinitionError::InvalidLevelName {
                        index,
                        name,
                        message: error.to_string(),
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(levels, raw.target, raw.source)
    }
}

fn validate_levels(levels: &[Identifier]) -> Result<(), LogDefinitionError> {
    if levels.is_empty() {
        return Err(LogDefinitionError::EmptyLevels);
    }
    if levels.len() > MAX_LEVELS {
        return Err(LogDefinitionError::TooManyLevels {
            count: levels.len(),
            max: MAX_LEVELS,
        });
    }
    let mut seen = HashSet::new();
    for level in levels {
        if !seen.insert(level) {
            return Err(LogDefinitionError::DuplicateLevel {
                name: level.clone(),
            });
        }
    }
    Ok(())
}

/// A level name paired with its zero-based rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankedLevel<'a> {
    name: &'a Identifier,
    rank: u8,
}

impl<'a> RankedLevel<'a> {
    /// Return the event and level name.
    pub const fn name(self) -> &'a Identifier {
        self.name
    }

    /// Return the zero-based severity rank.
    pub const fn rank(self) -> u8 {
        self.rank
    }
}

/// A malformed log definition payload.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LogDefinitionError {
    #[error("constraint data is missing")]
    MissingData,
    #[error("failed to decode log definition: {message}")]
    MalformedData { message: String },
    #[error("log levels must not be empty")]
    EmptyLevels,
    #[error("log declares {count} levels, exceeding the maximum of {max}")]
    TooManyLevels { count: usize, max: usize },
    #[error("log level {index} has invalid name {name:?}: {message}")]
    InvalidLevelName {
        index: usize,
        name: String,
        message: String,
    },
    #[error("log level {name:?} is declared more than once")]
    DuplicateLevel { name: Identifier },
}

/// Validates log-sink entity annotations against their generated events.
#[derive(Default)]
pub struct LogConstraint {
    errors: Vec<LogError>,
}

impl Visitor for LogConstraint {
    type Output = Result<(), LogError>;

    fn visit(&mut self, cursor: &Cursor) {
        match cursor.current() {
            Element::Entity(entity) => {
                let Some(definition) = LogDefinition::from_entity(entity) else {
                    return;
                };
                match definition {
                    Ok(definition) => check_entity(entity, &definition, &mut self.errors),
                    Err(source) => self.errors.push(LogError::InvalidData {
                        entity: entity.path().clone(),
                        source,
                    }),
                }
            }
            Element::Annotations(annotations)
                if !matches!(cursor.previous(), Some(Element::Entity(_)))
                    && annotations.has_constraint(Self::NAME) =>
            {
                self.errors.push(LogError::Misplaced {
                    location: cursor.to_string(),
                });
            }
            _ => {}
        }
    }

    fn finish(self) -> Self::Output {
        match self.errors.len() {
            0 => Ok(()),
            1 => Err(self.errors.into_iter().next().unwrap()),
            _ => Err(LogError::Multiple(self.errors)),
        }
    }
}

impl Constraint for LogConstraint {
    const NAME: &'static str = LogDefinition::NAME;
}

fn check_entity(entity: &Entity, definition: &LogDefinition, errors: &mut Vec<LogError>) {
    let levels: HashSet<_> = definition.levels.iter().collect();
    for level in &definition.levels {
        if entity.event(level).is_none() {
            errors.push(LogError::MissingLevelEvent {
                entity: entity.path().clone(),
                level: level.clone(),
            });
        }
    }
    for event in entity.events() {
        if !levels.contains(event.name()) {
            errors.push(LogError::UnexpectedEvent {
                entity: entity.path().clone(),
                event: event.name().clone(),
            });
            continue;
        }
        if event.cardinality() != Cardinality::Multi {
            errors.push(LogError::CardinalityMismatch {
                entity: entity.path().clone(),
                event: event.name().clone(),
                found: event.cardinality(),
            });
        }
        check_field(entity, event, "message", &DataType::String, errors);
        if definition.target {
            check_field(entity, event, "target", &optional(DataType::String), errors);
        }
        if definition.source.file {
            check_field(entity, event, "file", &optional(DataType::String), errors);
        }
        if definition.source.line {
            check_field(entity, event, "line", &optional(DataType::U32), errors);
        }
        if definition.source.module {
            check_field(entity, event, "module", &optional(DataType::String), errors);
        }
    }
}

fn optional(ty: DataType) -> DataType {
    DataType::Option(Box::new(ty))
}

fn check_field(
    entity: &Entity,
    event: &quent_schema::Event,
    name: &str,
    expected: &DataType,
    errors: &mut Vec<LogError>,
) {
    let name = Identifier::try_new(name).unwrap();
    match event.field(&name) {
        None => errors.push(LogError::MissingField {
            entity: entity.path().clone(),
            event: event.name().clone(),
            field: name,
            expected: type_name(expected),
        }),
        Some(field) if field.ty() != expected => errors.push(LogError::IncorrectFieldType {
            entity: entity.path().clone(),
            event: event.name().clone(),
            field: name,
            expected: type_name(expected),
            found: type_name(field.ty()),
        }),
        Some(_) => {}
    }
}

fn type_name(ty: &DataType) -> String {
    match ty {
        DataType::String => "string".to_string(),
        DataType::U32 => "u32".to_string(),
        DataType::Option(inner) => format!("option<{}>", type_name(inner)),
        other => format!("{other:?}"),
    }
}

/// A violation of the log-sink constraint.
#[derive(Debug, Error)]
pub enum LogError {
    #[error("entity \"{entity}\" log: invalid constraint data: {source}")]
    InvalidData {
        entity: Path,
        source: LogDefinitionError,
    },
    #[error("{location}: the log constraint is valid only on an entity")]
    Misplaced { location: String },
    #[error("entity \"{entity}\" log: level \"{level}\" does not match any event")]
    MissingLevelEvent { entity: Path, level: Identifier },
    #[error("entity \"{entity}\" log: event \"{event}\" is not a declared level")]
    UnexpectedEvent { entity: Path, event: Identifier },
    #[error("entity \"{entity}\" log: event \"{event}\" must be repeatable, found {found:?}")]
    CardinalityMismatch {
        entity: Path,
        event: Identifier,
        found: Cardinality,
    },
    #[error("entity \"{entity}\" log event \"{event}\" is missing `{field}: {expected}`")]
    MissingField {
        entity: Path,
        event: Identifier,
        field: Identifier,
        expected: String,
    },
    #[error(
        "entity \"{entity}\" log event \"{event}\" field `{field}` must be {expected}, found {found}"
    )]
    IncorrectFieldType {
        entity: Path,
        event: Identifier,
        field: Identifier,
        expected: String,
        found: String,
    },
    #[error("multiple log violations:\n{}", bullet_list(.0))]
    Multiple(Vec<LogError>),
}
