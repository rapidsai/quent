// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{AnalyzerError, AnalyzerResult, Entity, Span, resource::ResourceGroup};
use quent_events::Event;
use quent_query_engine_events::engine::{EngineEvent, EngineImplementationAttributes};
use quent_query_engine_ui as ui;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec, try_to_secs_relative};
use uuid::Uuid;

/// An [`Engine`] represents the top-level [`Entity`] of the query engine model.
///
/// Nothing should outlive the lifetime of an [`Engine`].
#[derive(Debug)]
pub struct Engine {
    /// The ID of this [`Engine`].
    pub id: Uuid,

    /// The time at which this [`Engine`] started.
    pub start_time_unix_ns: Option<TimeUnixNanoSec>,
    /// The time at which this engine exited.
    pub end_time_unix_ns: Option<TimeUnixNanoSec>,

    /// The name of this [`Engine`] instance.
    pub instance_name: Option<String>,
    /// Details about the Engine implementation.
    pub implementation: Option<EngineImplementationAttributes>,
}

impl Engine {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            start_time_unix_ns: None,
            end_time_unix_ns: None,
            instance_name: None,
            implementation: None,
        }
    }

    pub fn push(&mut self, event: Event<EngineEvent>) {
        match event.data {
            EngineEvent::Init(init) => {
                self.start_time_unix_ns = Some(event.timestamp);
                self.instance_name = init.instance_name;
                self.implementation = init.implementation;
            }
            EngineEvent::Exit => self.end_time_unix_ns = Some(event.timestamp),
        }
    }

    pub fn to_ui(&self) -> AnalyzerResult<ui::Engine> {
        let duration_s = if let Some(start) = self.start_time_unix_ns
            && let Some(end) = self.end_time_unix_ns
        {
            Some(try_to_secs_relative(end, start)?)
        } else {
            None
        };

        Ok(ui::Engine {
            id: self.id,
            start_time_unix_ns: self.start_time_unix_ns,
            duration_s,
            instance_name: self.instance_name.clone(),
            implementation: self.implementation.as_ref().map(|i| i.into()),
        })
    }
}

impl Entity for Engine {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "engine"
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or_default()
    }
}

impl Span for Engine {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        if let Some(init) = self.start_time_unix_ns
            && let Some(exit) = self.end_time_unix_ns
        {
            Ok(SpanUnixNanoSec::try_new(init, exit)?)
        } else {
            Err(AnalyzerError::IncompleteEntity(
                "engine does not have an exit timestamp".to_string(),
            ))
        }
    }
}

impl ResourceGroup for Engine {
    fn parent_group_id(&self) -> Option<Uuid> {
        None
    }
}
