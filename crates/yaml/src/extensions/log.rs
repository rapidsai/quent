// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Log-sink forms and their schema elaboration.

use indexmap::IndexMap;
use quent_log::{LevelDecl, LogEntityBuilder, SourceFields};
use quent_schema::builder::AnnotationsBuilder;
use quent_schema::{Annotations, Entity, Identifier};
use serde::Deserialize;

use crate::ast;
use crate::diag::Diagnostics;
use crate::extensions::{Elaborator, EventContext};
use crate::lower::{build_or_diagnose, event_fields, ident};

/// Deserialize a present optional field without accepting YAML `null` as
/// equivalent to absence.
pub(crate) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Entity events together with whether the `events:` key was present.
///
/// Presence is retained so an empty `events: {}` still conflicts with `log:`.
#[derive(Debug, Default)]
pub(crate) struct EventMap {
    pub(crate) present: bool,
    pub(crate) entries: IndexMap<String, ast::Event>,
}

impl<'de> Deserialize<'de> for EventMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            present: true,
            entries: IndexMap::deserialize(deserializer)?,
        })
    }
}

/// A log sink: standard fields, common attributes, and ordered levels.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LogSpec {
    #[serde(default)]
    target: bool,
    #[serde(default, deserialize_with = "present")]
    source: Option<LogSource>,
    #[serde(default)]
    attributes: IndexMap<String, ast::Field>,
    levels: Vec<LogLevel>,
}

/// Source-field selection, either all fields or an independent selection.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LogSource {
    All(bool),
    Detailed(LogSourceFields),
}

/// Individually selected log source fields.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogSourceFields {
    #[serde(default)]
    file: bool,
    #[serde(default)]
    line: bool,
    #[serde(default)]
    module: bool,
}

/// One log level and its event-specific additions.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogLevel {
    name: String,
    #[serde(default)]
    doc: Option<String>,
    #[serde(default)]
    attributes: IndexMap<String, ast::Field>,
}

/// Elaborate a `log:` block into one repeatable event per declared level.
pub(crate) fn elaborate(
    id: Identifier,
    spec: &LogSpec,
    annotations: Annotations,
    entity_path: &str,
    event_context: &EventContext,
    extensions: &Elaborator,
    sink: &mut Diagnostics,
) -> Option<Entity> {
    let log_path = format!("{entity_path}.log");
    let source = match &spec.source {
        None | Some(LogSource::All(false)) => SourceFields::default(),
        Some(LogSource::All(true)) => SourceFields::all(),
        Some(LogSource::Detailed(source)) => {
            SourceFields::new(source.file, source.line, source.module)
        }
    };
    let common = event_fields(
        &spec.attributes,
        &format!("{log_path}.attributes"),
        event_context,
        extensions,
        sink,
    );

    let mut levels = Vec::new();
    let mut complete = true;
    for (rank, level) in spec.levels.iter().enumerate() {
        let level_path = format!("{log_path}.levels.{rank}");
        let Some(name) = ident(&level.name, &format!("{level_path}.name"), sink) else {
            complete = false;
            continue;
        };
        let attributes = event_fields(
            &level.attributes,
            &format!("{level_path}.attributes"),
            event_context,
            extensions,
            sink,
        );
        let event_annotations = match &level.doc {
            Some(doc) => AnnotationsBuilder::new().with_docs(doc).build(),
            None => AnnotationsBuilder::new().build(),
        };
        let Some(event_annotations) = build_or_diagnose(event_annotations, &level_path, sink)
        else {
            complete = false;
            continue;
        };
        levels.push(LevelDecl {
            name,
            annotations: event_annotations,
            attributes,
        });
    }
    if !complete {
        return None;
    }

    match LogEntityBuilder::new(id)
        .with_annotations(annotations)
        .with_target(spec.target)
        .with_source(source)
        .with_attributes(common)
        .with_levels(levels)
        .build()
    {
        Ok(entity) => Some(entity),
        Err(error) => {
            sink.error(&log_path, error.to_string(), None);
            None
        }
    }
}
