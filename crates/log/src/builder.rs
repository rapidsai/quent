// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use quent_constraints::Constraint as _;
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Field, Identifier, Path,
    builder::{AnnotationsBuilder, BuilderError, EntityBuilder, EventBuilder},
};
use thiserror::Error;

use crate::{LogConstraint, LogDefinition, LogDefinitionError, SourceFields, check_entity};

/// One level used to build a log-sink entity.
pub struct LevelDecl {
    /// Level and generated event name.
    pub name: Identifier,
    /// Annotations attached to the generated event.
    pub annotations: Annotations,
    /// Fields specific to this level.
    pub attributes: Vec<Field>,
}

/// Builds a log-sink entity with one repeatable event per level.
pub struct LogEntityBuilder {
    path: Path,
    annotations: AnnotationsBuilder,
    target: bool,
    source: SourceFields,
    attributes: Vec<Field>,
    levels: Vec<LevelDecl>,
}

impl LogEntityBuilder {
    /// Begin a log-sink entity at `path`.
    pub fn new(path: impl Into<Path>) -> Self {
        Self {
            path: path.into(),
            annotations: AnnotationsBuilder::new(),
            target: false,
            source: SourceFields::default(),
            attributes: Vec::new(),
            levels: Vec::new(),
        }
    }

    /// Set entity annotations, preserving them alongside the log constraint.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Enable or disable the optional `target` field.
    pub fn with_target(mut self, target: bool) -> Self {
        self.target = target;
        self
    }

    /// Select the optional source fields.
    pub fn with_source(mut self, source: SourceFields) -> Self {
        self.source = source;
        self
    }

    /// Add fields shared by every generated level event.
    pub fn with_attributes(mut self, attributes: impl IntoIterator<Item = Field>) -> Self {
        self.attributes.extend(attributes);
        self
    }

    /// Add a level in rank order.
    pub fn with_level(mut self, level: LevelDecl) -> Self {
        self.levels.push(level);
        self
    }

    /// Add several levels in rank order.
    pub fn with_levels(mut self, levels: impl IntoIterator<Item = LevelDecl>) -> Self {
        self.levels.extend(levels);
        self
    }

    /// Assemble and validate the log-sink entity.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid level list, an attribute collision, an
    /// invalid generated schema element, or serialization failure.
    pub fn build(self) -> Result<Entity, LogEntityBuilderError> {
        let Self {
            path,
            mut annotations,
            target,
            source,
            attributes,
            levels,
        } = self;

        let definition = LogDefinition::new(
            levels.iter().map(|level| level.name.clone()).collect(),
            target,
            source,
        )?;
        validate_attributes(&attributes, &levels, target, source)?;

        let mut entity = EntityBuilder::new(path);
        for level in levels {
            let fields = standard_fields(target, source)
                .into_iter()
                .chain(attributes.iter().cloned())
                .chain(level.attributes);
            let event = EventBuilder::new(level.name, Cardinality::Multi)
                .with_fields(fields)
                .with_annotations(level.annotations)
                .build()?;
            entity = entity.with_event(event);
        }

        annotations =
            annotations.with_constraint(LogConstraint::NAME, Some(definition.constraint_data()?));
        let entity = entity.with_annotations(annotations.build()?).build()?;
        let mut errors = Vec::new();
        check_entity(&entity, &definition, &mut errors);
        if let Some(error) = errors.into_iter().next() {
            Err(LogEntityBuilderError::Invalid(Box::new(error)))
        } else {
            Ok(entity)
        }
    }
}

fn validate_attributes(
    common: &[Field],
    levels: &[LevelDecl],
    target: bool,
    source: SourceFields,
) -> Result<(), LogEntityBuilderError> {
    let implicit: HashSet<_> = standard_fields(target, source)
        .into_iter()
        .map(|field| field.name().clone())
        .collect();
    let mut common_names = HashSet::new();
    for field in common {
        if implicit.contains(field.name()) || !common_names.insert(field.name()) {
            return Err(LogEntityBuilderError::CommonAttributeCollision {
                name: field.name().clone(),
            });
        }
    }
    for level in levels {
        let mut names = HashSet::new();
        for field in &level.attributes {
            if implicit.contains(field.name())
                || common_names.contains(field.name())
                || !names.insert(field.name())
            {
                return Err(LogEntityBuilderError::LevelAttributeCollision {
                    level: level.name.clone(),
                    name: field.name().clone(),
                });
            }
        }
    }
    Ok(())
}

fn standard_fields(target: bool, source: SourceFields) -> Vec<Field> {
    let field = |name: &str, ty| {
        Field::new(
            Identifier::try_new(name).unwrap(),
            ty,
            Annotations::default(),
        )
    };
    let optional = |ty| DataType::Option(Box::new(ty));
    let mut fields = vec![field("message", DataType::String)];
    if target {
        fields.push(field("target", optional(DataType::String)));
    }
    if source.file() {
        fields.push(field("file", optional(DataType::String)));
    }
    if source.line() {
        fields.push(field("line", optional(DataType::U32)));
    }
    if source.module() {
        fields.push(field("module", optional(DataType::String)));
    }
    fields
}

/// A problem that prevents building a valid log-sink entity.
#[derive(Debug, Error)]
pub enum LogEntityBuilderError {
    /// The log definition is invalid.
    #[error(transparent)]
    Definition(#[from] LogDefinitionError),
    /// A user attribute conflicts with another generated event field.
    #[error("common log attribute `{name}` conflicts with another event field")]
    CommonAttributeCollision { name: Identifier },
    /// A level attribute conflicts with a common or implicit field.
    #[error("log level `{level}` attribute `{name}` conflicts with another event field")]
    LevelAttributeCollision { level: Identifier, name: Identifier },
    /// A generated schema element is invalid.
    #[error(transparent)]
    Build(#[from] BuilderError),
    /// The constraint payload failed to serialize.
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
    /// The assembled entity does not satisfy the log constraint.
    #[error(transparent)]
    Invalid(Box<crate::LogError>),
}
