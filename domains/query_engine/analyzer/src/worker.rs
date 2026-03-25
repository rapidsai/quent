// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{AnalyzerError, AnalyzerResult, Entity, Span, resource::ResourceGroup};
use quent_events::Event;
use quent_query_engine_events::worker::WorkerEvent;
use quent_query_engine_ui as ui;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

/// A [`Worker`] is an [`Entity`] that executes [`Query`] [`Plans`].
#[derive(Debug)]
pub struct Worker {
    /// The ID of this [`Worker`].
    pub id: Uuid,
    /// The ID of the [`Engine`] to which this [`Worker`] belongs.
    pub parent_engine_id: Option<Uuid>,
    /// The name of this [`Worker`].
    pub instance_name: Option<String>,

    /// The time at which this [`Worker`] started.
    pub start_unix_ns: Option<TimeUnixNanoSec>,
    /// The time at which this [`Worker`] exited.
    pub end_unix_ns: Option<TimeUnixNanoSec>,
}

impl Worker {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "worker id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                parent_engine_id: None,
                instance_name: None,
                start_unix_ns: None,
                end_unix_ns: None,
            })
        }
    }

    pub fn push(&mut self, event: Event<WorkerEvent>) {
        match event.data {
            WorkerEvent::Init(init) => {
                self.start_unix_ns = Some(event.timestamp);
                self.parent_engine_id = Some(init.parent_engine_id);
                self.instance_name = Some(init.instance_name);
            }
            WorkerEvent::Exit => self.end_unix_ns = Some(event.timestamp),
        }
    }

    pub fn to_ui(&self, _epoch: TimeUnixNanoSec) -> ui::Worker {
        ui::Worker {
            id: self.id,
            parent_engine_id: self.parent_engine_id,
            instance_name: self.instance_name.clone(),
            start_unix_ns: self.start_unix_ns,
            end_unix_ns: self.end_unix_ns,
        }
    }
}

impl Entity for Worker {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "worker"
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or_default()
    }
}

impl Span for Worker {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        if let Some(start) = self.start_unix_ns
            && let Some(end) = self.end_unix_ns
        {
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
        self.parent_engine_id
    }
}
