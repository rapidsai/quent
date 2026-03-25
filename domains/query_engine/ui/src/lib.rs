// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Types shared with the UI.

use quent_analyzer::fsm::FsmTypeDecl;
use quent_attributes::{Attribute, Value};
use quent_query_engine_events as qe;
use quent_time::{SpanSec, TimeSec, TimeUnixNanoSec};
use quent_ui::{
    Resource, ResourceGroup, ResourceGroupTypeDecl, ResourceTree, ResourceTypeDecl,
    quantity::QuantitySpec,
};
use serde::Serialize;
use std::collections::HashMap;
use ts_rs::TS;
use uuid::Uuid;

/// Attributes describing details about the implementation of this Engine
#[derive(TS, Debug, Serialize)]
pub struct EngineImplementationAttributes {
    /// The name of this Engine implementation, e.g. "SiriusDB", "Velox", "DataFusion", etc.
    pub name: Option<String>,
    /// The version of this Engine implementation, e.g. "13.3.7"
    pub version: Option<String>,
    /// Arbitrary attributes defined at run time.
    pub custom_attributes: Vec<Attribute>,
}

impl From<&qe::engine::EngineImplementationAttributes> for EngineImplementationAttributes {
    fn from(value: &qe::engine::EngineImplementationAttributes) -> Self {
        Self {
            name: value.name.clone(),
            version: value.version.clone(),
            custom_attributes: value.custom_attributes.clone(),
        }
    }
}

/// The engine that executed a [`Query`].
#[derive(TS, Debug, Serialize)]
pub struct Engine {
    /// The ID of this [`Engine`].
    pub id: Uuid,
    /// The timestamp at which this [`Engine`] started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time_unix_ns: Option<TimeUnixNanoSec>,
    /// The duration for which this [`Engine`] was alive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_s: Option<TimeSec>,
    /// The name of this [`Engine`] instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_name: Option<String>,
    /// Details about the Engine implementation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<EngineImplementationAttributes>,
}

impl Engine {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            start_time_unix_ns: None,
            duration_s: None,
            instance_name: None,
            implementation: None,
        }
    }
}

/// A group of [`Query`]s.
#[derive(TS, Debug, Serialize)]
pub struct QueryGroup {
    /// The ID of this query group.
    pub id: Uuid,
    /// The name of this query group.
    pub instance_name: Option<String>,
    /// The id of the engine this query group was executed on.
    pub engine_id: Option<Uuid>,
}

/// A [`Query`] executed by an [`Engine`].
#[derive(TS, Debug, Serialize)]
pub struct Query {
    /// The ID of this [`Query`].
    pub id: Uuid,
    /// The ID of the [`super::query_group::QueryGroup`] this query is part of.
    pub query_group_id: Uuid,
    /// A name for this [`Query`].
    pub instance_name: Option<String>,

    /// The start time of this query, relative to the Unix epoch.
    pub start_unix_ns: Option<TimeUnixNanoSec>,

    /// The time relative to the start time at which the engine started planning
    /// this query.
    pub planning_s: Option<TimeSec>,
    /// The time relative to the start time at which the engine started
    /// executing this query, after planning.
    pub executing_s: Option<TimeSec>,
    /// The time relative to the start time at which the engine started
    /// completed executing this query.
    pub completed_s: Option<TimeSec>,
}

/// A worker that executed a leaf [`Plan`].
#[derive(TS, Debug, Serialize)]
pub struct Worker {
    /// The ID of this [`Worker`].
    pub id: Uuid,
    /// The ID of the [`Engine`] to which this [`Worker`] belongs.
    pub parent_engine_id: Option<Uuid>,
    /// The name of this [`Worker`].
    pub instance_name: Option<String>,

    /// The time at which this [`Worker`] started, relative to the engine.
    pub start_unix_ns: Option<TimeUnixNanoSec>,
    /// The time at which this [`Worker`] exited.
    pub end_unix_ns: Option<TimeUnixNanoSec>,
}

/// An edge between two [`Operator`] [`Port`]s.
#[derive(TS, Debug, Serialize)]
pub struct Edge {
    /// The [`Port`] that produced the data flowing over this edge.
    pub source: Uuid,
    /// The [`Port`] that consumed the data flowing over this edge.
    pub target: Uuid,
}

/// A plan for executing a [`Query`].
///
/// The topology of the plan is a Directed Acyclic Graph (DAG).
#[derive(TS, Debug, Serialize)]
pub struct Plan {
    /// The ID of this [`Plan`].
    pub id: Uuid,
    /// The name of this [`Plan`].
    pub instance_name: Option<String>,
    /// The ID of the parent [`Plan`], if any.
    pub parent: Option<Uuid>,
    /// The ID of the [`super::worker::Worker`] that executed this [`Plan`].
    ///
    /// If this level of [`Plan`] was not directly executed by a [`Worker`],
    /// then this is set to None.
    pub worker_id: Option<Uuid>,
    /// The [`Edge`]s between [`Operator`] [`Port`]s of this [`Plan`].
    pub edges: Vec<Edge>,
}

#[derive(TS, Debug, Serialize)]
pub struct OperatorStatistics {
    /// Custom statistics
    pub custom_statistics: HashMap<String, Option<Value>>,
}

#[derive(TS, Debug, Serialize)]
pub struct Operator {
    /// The ID of this [`Operator`].
    pub id: Uuid,
    /// The ID of the [`Plan`] this [`Operator`] belongs to.
    pub plan_id: Option<Uuid>,
    /// A list of [`Operator`] IDs in a parent plan (if any) from which this
    /// [`Operator`] was derived.
    pub parent_operator_ids: Vec<Uuid>,
    /// The name of this [`Operator`].
    pub instance_name: Option<String>,
    /// The name of this type of [`Operator`].
    pub operator_type_name: Option<String>,

    /// The custom attributes of this [`Operator`].
    pub custom_attributes: HashMap<String, Option<Value>>,
    /// The statistics of this [`Operator`].
    ///
    /// These are attributes that are typically gathered after the work
    /// described by an [`Operator`] has completed.
    pub statistics: Option<OperatorStatistics>,

    /// The span of time between the first moment an operator started processing
    /// an input, and the latest moment at which an operator finished producing
    /// an output (excluding any potential back-pressure).
    ///
    /// There may have been gaps in this span in which this operator was not
    /// actively using any resources. Thus, this span of time does NOT represent
    /// e.g. "CPU time" spent.
    pub active_span: Option<SpanSec>,
}

#[derive(TS, Debug, Serialize)]
pub struct PortStatistics {
    /// Custom statistics
    pub custom_statistics: HashMap<String, Option<Value>>,
}

#[derive(TS, Debug, Serialize)]
pub struct Port {
    /// The ID of this [`Port`]
    pub id: Uuid,
    /// The [`Operator`] to which this [`Port`] belongs.
    pub operator_id: Option<Uuid>,
    /// The name of this [`Port`].
    pub instance_name: Option<String>,
    /// Statistics associated with this port:
    pub statistics: Option<PortStatistics>,
}

#[derive(TS, Debug, Serialize)]
pub struct PlanTree {
    /// The ID of the plan at this node in the tree.
    pub id: Uuid,
    /// The ID of the worker that this Plan was local to, if any.
    pub worker: Option<Uuid>,
    /// The children of the plan at the node in this tree.
    pub children: Vec<PlanTree>,
}

#[derive(TS, Debug, Serialize)]
pub struct QueryEntities {
    /// The engine that ran this query.
    ///
    /// Is a Resource Group.
    pub engine: Engine,
    /// The group of this query.
    ///
    /// Is a Resource Group.
    pub query_group: QueryGroup,
    /// The query.
    ///
    /// Is a Resource Group.
    pub query: Query,
    /// The workers of this query.
    ///
    /// Is a Resource Group.
    pub workers: HashMap<Uuid, Worker>,
    /// The plans of this query.
    ///
    /// Is a Resource Group.
    pub plans: HashMap<Uuid, Plan>,
    /// The operators of the plans.
    ///
    /// Is a Resource Group.
    pub operators: HashMap<Uuid, Operator>,
    /// The ports of the operators.
    ///
    /// Is a Resource Group.
    pub ports: HashMap<Uuid, Port>,

    /// Application-specific resource types
    pub resource_types: HashMap<String, ResourceTypeDecl>,
    /// Resource group types.
    ///
    /// This includes declarations for:
    /// - [`Engine`]
    /// - [`QueryGroup`]
    /// - [`Query`]
    /// - [`Worker`]
    /// - [`Plan`]
    /// - [`Operator`]
    /// - [`Port`]
    pub resource_group_types: HashMap<String, ResourceGroupTypeDecl>,
    /// Application-specific resources
    pub resources: HashMap<Uuid, Resource>,
    /// Application-specific resource groups
    pub resource_groups: HashMap<Uuid, ResourceGroup>,

    /// Application-specific FSMs
    pub fsm_types: HashMap<String, FsmTypeDecl>,
}

#[derive(TS, Serialize)]
pub struct QueryBundle<E> {
    /// The ID of the query.
    pub query_id: Uuid,
    /// Maps with entities that are involved in this query.
    pub entities: QueryEntities,

    /// A tree of plans involved in the execution of this query.
    pub plan_tree: PlanTree,
    /// A tree of resources involved in the execution of this query.
    pub resource_tree: ResourceTree<E>,

    /// A list of unique operator type names.
    pub unique_operator_names: Vec<String>,

    /// Quantity specifications for capacity display, keyed by capacity name.
    pub quantity_specs: HashMap<String, QuantitySpec>,

    /// The number of nanoseconds passed since the Unix epoch at which the
    /// engine started executing this query.
    pub start_time_unix_ns: TimeUnixNanoSec,
    /// The duration of this query, in seconds.
    pub duration_s: TimeSec,
}
