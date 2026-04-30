// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::entity::EntityEvents;
use quent_analyzer::{AnalyzerError, AnalyzerResult, Entity, Span, resource::ResourceGroup};
use quent_events::Event;
use quent_query_engine_model::worker;
use quent_query_engine_ui as ui;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

/// A [`Worker`] is an [`Entity`] that executes `Query` `Plan`s.
#[derive(Debug)]
pub struct Worker(EntityEvents<worker::Worker>);

impl Worker {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        Ok(Self(EntityEvents::new(id)?))
    }

    pub fn push(&mut self, event: Event<worker::WorkerEvent>) {
        self.0.push(event);
    }

    pub fn to_ui(&self, _epoch: TimeUnixNanoSec) -> ui::Worker {
        let d = self.0.data();
        ui::Worker {
            id: self.0.id(),
            parent_engine_id: d.init.as_ref().map(|i| i.parent_engine_id.uuid()),
            instance_name: d.init.as_ref().map(|i| i.instance_name.clone()),
            start_unix_ns: self.0.earliest_timestamp(),
            end_unix_ns: self.0.latest_timestamp(),
        }
    }
}

impl Entity for Worker {
    fn id(&self) -> Uuid {
        self.0.id()
    }
    fn type_name(&self) -> &str {
        "worker"
    }
    fn instance_name(&self) -> &str {
        self.0
            .data()
            .init
            .as_ref()
            .map(|i| i.instance_name.as_str())
            .unwrap_or_default()
    }
}

impl Span for Worker {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        if let (Some(start), Some(end)) = (self.0.earliest_timestamp(), self.0.latest_timestamp()) {
            Ok(SpanUnixNanoSec::try_new(start, end)?)
        } else {
            Err(AnalyzerError::IncompleteEntity(
                "worker does not have an init or exit timestamp".to_string(),
            ))
        }
    }
}

impl ResourceGroup for Worker {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.0
            .data()
            .init
            .as_ref()
            .map(|i| i.parent_engine_id.uuid())
    }
}
