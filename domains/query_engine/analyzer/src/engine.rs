// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::entity::EntityEvents;
use quent_analyzer::{AnalyzerError, AnalyzerResult, Entity, Span, resource::ResourceGroup};
use quent_events::Event;
use quent_query_engine_model::engine;
use quent_query_engine_ui as ui;
use quent_time::{span::SpanUnixNanoSec, try_to_secs_relative};
use uuid::Uuid;

/// The analyzer's Engine entity.
#[derive(Debug)]
pub struct Engine(EntityEvents<engine::Engine>);

impl Engine {
    pub fn new(id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self(EntityEvents::new(id)?))
    }

    pub fn push(&mut self, event: Event<engine::EngineEvent>) {
        self.0.push(event);
    }

    pub fn to_ui(&self) -> AnalyzerResult<ui::Engine> {
        let d = self.0.data();
        let start = self.0.earliest_timestamp();
        let end = self.0.latest_timestamp();

        let duration_s = if let (Some(s), Some(e)) = (start, end) {
            Some(try_to_secs_relative(e, s)?)
        } else {
            None
        };

        Ok(ui::Engine {
            id: self.0.id(),
            start_time_unix_ns: start,
            duration_s,
            instance_name: d.init.as_ref().and_then(|i| i.instance_name.clone()),
            implementation: d.init.as_ref().map(|i| (&i.implementation).into()),
        })
    }
}

impl Entity for Engine {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "engine"
    }
    fn instance_name(&self) -> &str {
        self.0
            .data()
            .init
            .as_ref()
            .and_then(|i| i.instance_name.as_deref())
            .unwrap_or_default()
    }
}

impl Span for Engine {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        if let (Some(start), Some(end)) = (self.0.earliest_timestamp(), self.0.latest_timestamp()) {
            Ok(SpanUnixNanoSec::try_new(start, end)?)
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
