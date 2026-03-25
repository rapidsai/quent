// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, Transition},
    resource::ResourceGroup,
};
use quent_attributes::Attribute;
use quent_events::Event;
use quent_query_engine_events::query::QueryEvent;
use quent_query_engine_ui as ui;
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec, Timestamp, try_to_secs_relative};
use uuid::Uuid;

#[derive(Debug)]
pub enum QueryTransition {
    Init(TimeUnixNanoSec),
    Planning(TimeUnixNanoSec),
    Executing(TimeUnixNanoSec),
    Exit(TimeUnixNanoSec),
}

impl Timestamp for QueryTransition {
    fn timestamp(&self) -> TimeUnixNanoSec {
        *match self {
            QueryTransition::Init(ts) => ts,
            QueryTransition::Planning(ts) => ts,
            QueryTransition::Executing(ts) => ts,
            QueryTransition::Exit(ts) => ts,
        }
    }
}

impl Transition for QueryTransition {
    fn name(&self) -> &str {
        match self {
            QueryTransition::Init(_) => "init",
            QueryTransition::Planning(_) => "planning",
            QueryTransition::Executing(_) => "executing",
            QueryTransition::Exit(_) => "exit",
        }
    }
    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        std::iter::empty()
    }
}

/// A query executed by an [`super::engine::Engine`].
#[derive(Debug)]
pub struct Query {
    /// The ID of this [`Query`].
    pub id: Uuid,
    /// The ID of the [`super::query_group::QueryGroup`] this query is part of.
    pub query_group_id: Uuid,
    /// A name for this [`Query`].
    pub instance_name: Option<String>,
    /// The sequence of state transitions this [`Query`] went through.
    pub transitions: [QueryTransition; 4],
}

pub struct QueryBuilder {
    pub id: Uuid,
    pub query_group_id: Option<Uuid>,
    pub instance_name: Option<String>,
    pub transitions: TimeOrderedCollector<QueryTransition>,
}

impl QueryBuilder {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "query id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                query_group_id: None,
                instance_name: None,
                transitions: Default::default(),
            })
        }
    }

    pub fn push(&mut self, event: Event<QueryEvent>) {
        match event.data {
            QueryEvent::Init(init) => {
                self.transitions
                    .push(QueryTransition::Init(event.timestamp));
                self.instance_name = Some(init.instance_name);
                self.query_group_id = Some(init.query_group_id);
            }
            QueryEvent::Planning => {
                self.transitions
                    .push(QueryTransition::Planning(event.timestamp));
            }
            QueryEvent::Executing => {
                self.transitions
                    .push(QueryTransition::Executing(event.timestamp));
            }
            QueryEvent::Exit => {
                self.transitions
                    .push(QueryTransition::Exit(event.timestamp));
            }
        }
    }

    pub fn try_build(self) -> AnalyzerResult<Query> {
        let transitions = self.transitions.into_inner();

        // Validate and convert to array:
        let transitions: [QueryTransition; 4] = transitions.try_into().map_err(|v: Vec<_>| {
            AnalyzerError::Validation(format!(
                "query fsm expects exactly four transitions, got {}",
                v.len()
            ))
        })?;

        match (
            &transitions[0],
            &transitions[1],
            &transitions[2],
            &transitions[3],
        ) {
            (
                QueryTransition::Init(_),
                QueryTransition::Planning(_),
                QueryTransition::Executing(_),
                QueryTransition::Exit(_),
            ) => Ok(Query {
                id: self.id,
                query_group_id: self.query_group_id.ok_or_else(|| {
                    AnalyzerError::Validation(format!("query {} has no group id", self.id,))
                })?,
                instance_name: self.instance_name,
                transitions,
            }),
            _ => Err(AnalyzerError::Validation(format!(
                "query fsm expected to go through states: init -> planning -> executing -> exit, went through: {:?}",
                transitions
            ))),
        }
    }
}

impl Query {
    pub fn to_ui(&self) -> AnalyzerResult<ui::Query> {
        let mut start_unix_ns = None;
        let mut planning_s = None;
        let mut executing_s = None;
        let mut completed_s = None;

        if let Some(init) = self.transition(0) {
            start_unix_ns = Some(init.timestamp());

            for i in 0..self.len() {
                if let Some(transition) = self.transition(i) {
                    match transition {
                        QueryTransition::Planning(ts) => {
                            planning_s = Some(try_to_secs_relative(*ts, init.timestamp())?);
                        }
                        QueryTransition::Executing(ts) => {
                            executing_s = Some(try_to_secs_relative(*ts, init.timestamp())?);
                            if let Some(exit) = self.transition(i + 1) {
                                completed_s =
                                    Some(try_to_secs_relative(exit.timestamp(), init.timestamp())?);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(ui::Query {
            id: self.id,
            query_group_id: self.query_group_id,
            instance_name: self.instance_name.clone(),
            start_unix_ns,
            planning_s,
            executing_s,
            completed_s,
        })
    }
}

impl Entity for Query {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "query"
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or_default()
    }
}

impl Fsm for Query {
    type TransitionType = QueryTransition;

    fn len(&self) -> usize {
        self.transitions.len() - 1 // -1 for the exit transition.
    }

    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.transitions.get(index)
    }
}

impl ResourceGroup for Query {
    fn parent_group_id(&self) -> Option<Uuid> {
        Some(self.query_group_id)
    }
}
