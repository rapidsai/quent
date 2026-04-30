// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Query FSM analysis.
//!
//! Uses `FsmEvents<QueryTransition>` for generic trait impls. A newtype
//! wrapper adds `ResourceGroup` and application-specific methods.

use quent_analyzer::{
    AnalyzerResult, Entity,
    fsm::{
        Fsm, FsmUsages,
        events::{FsmEvents, FsmEventsBuilder, TransitionEvent},
    },
    resource::{ResourceGroup, Usage, Using},
};
use quent_query_engine_model::query::QueryTransition as ModelQueryTransition;
use quent_query_engine_ui as ui;
use quent_time::{Timestamp, try_to_secs_relative};
use uuid::Uuid;

/// Builder for Query FSMs.
pub type QueryBuilder = FsmEventsBuilder<ModelQueryTransition>;

/// A reconstructed Query FSM with resource group support.
#[derive(Debug)]
pub struct Query {
    inner: FsmEvents<ModelQueryTransition>,
}

impl Query {
    pub fn from_builder(builder: QueryBuilder) -> AnalyzerResult<Self> {
        Ok(Self {
            inner: builder.try_build()?,
        })
    }

    pub fn query_group_id(&self) -> Option<Uuid> {
        self.inner.first_data().and_then(|t| match t {
            ModelQueryTransition::Init(init) => Some(init.query_group_id.uuid()),
            _ => None,
        })
    }

    pub fn to_ui(&self) -> AnalyzerResult<ui::Query> {
        let transitions = self.inner.transitions();
        let epoch = transitions.first().map(|t| t.timestamp());

        let mut start_unix_ns = None;
        let mut planning_s = None;
        let mut executing_s = None;
        let mut completed_s = None;

        if let Some(epoch) = epoch {
            start_unix_ns = Some(epoch);
            for (i, t) in transitions.iter().enumerate() {
                match &t.data {
                    ModelQueryTransition::Planning(_) => {
                        planning_s = Some(try_to_secs_relative(t.timestamp(), epoch)?);
                    }
                    ModelQueryTransition::Executing(_) => {
                        executing_s = Some(try_to_secs_relative(t.timestamp(), epoch)?);
                        if let Some(next) = transitions.get(i + 1) {
                            completed_s = Some(try_to_secs_relative(next.timestamp(), epoch)?);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(ui::Query {
            id: self.id(),
            query_group_id: self.query_group_id().unwrap_or(Uuid::nil()),
            instance_name: Some(self.instance_name().to_owned()).filter(|s| !s.is_empty()),
            start_unix_ns,
            planning_s,
            executing_s,
            completed_s,
        })
    }
}

// Delegate Entity (override instance_name for backward compat)

impl Entity for Query {
    fn id(&self) -> Uuid {
        self.inner.id()
    }
    fn type_name(&self) -> &str {
        self.inner.type_name()
    }
    fn instance_name(&self) -> &str {
        self.inner.instance_name()
    }
}

// Delegate Fsm

impl Fsm for Query {
    type TransitionType = TransitionEvent<ModelQueryTransition>;
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        Fsm::transition(&self.inner, index)
    }
}

// Delegate FsmUsages

impl<'a> FsmUsages<'a> for Query {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
        FsmUsages::usages_with_state_names(&self.inner)
    }
}

// Delegate Using

impl Using for Query {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        Using::usages(&self.inner)
    }
}

// ResourceGroup (application-specific)

impl ResourceGroup for Query {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.query_group_id()
    }
}
