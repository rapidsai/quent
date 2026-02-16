//! Types shared with the UI.

use quent_analyzer::fsm::State;
use quent_analyzer::{AnalyzerError, Entity};
use quent_attributes::{Attribute, Value};
use quent_query_engine_analyzer::{self as qa, query::QueryState};
use quent_query_engine_events as qe;
use quent_time::{TimeSec, TimeUnixNanoSec, try_to_secs_relative};
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
    pub start_time_unix_ns: Option<TimeUnixNanoSec>,
    /// The duration for which this [`Engine`] was alive.
    pub duration_s: Option<TimeSec>,
    /// The name of this [`Engine`] instance.
    pub instance_name: Option<String>,
    /// Details about the Engine implementation.
    pub implementation: Option<EngineImplementationAttributes>,
}

impl TryFrom<&qa::engine::Engine> for Engine {
    type Error = AnalyzerError;

    fn try_from(engine: &qa::engine::Engine) -> Result<Self, Self::Error> {
        let duration_s = if let Some(start) = engine.start_time_unix_ns
            && let Some(end) = engine.end_time_unix_ns
        {
            Some(try_to_secs_relative(end, start)?)
        } else {
            None
        };

        Ok(Self {
            id: engine.id,
            start_time_unix_ns: engine.start_time_unix_ns,
            duration_s,
            instance_name: engine.instance_name.clone(),
            implementation: engine.implementation.as_ref().map(|i| i.into()),
        })
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

impl From<&qa::query_group::QueryGroup> for QueryGroup {
    fn from(query_group: &qa::query_group::QueryGroup) -> Self {
        Self {
            id: query_group.id(),
            instance_name: query_group.instance_name.clone(),
            engine_id: query_group.engine_id,
        }
    }
}

/// A [`Query`] executed by an [`Engine`].
#[derive(TS, Debug, Serialize)]
pub struct Query {
    /// The ID of this [`Query`].
    pub id: Uuid,
    /// The ID of the [`super::query_group::QueryGroup`] this query is part of.
    pub query_group_id: Option<Uuid>,
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

impl TryFrom<&qa::query::Query> for Query {
    type Error = AnalyzerError;

    fn try_from(query: &qa::query::Query) -> Result<Self, Self::Error> {
        let mut start_unix_ns = None;
        let mut planning_s = None;
        let mut executing_s = None;
        let mut completed_s = None;

        if let Some(init) = query.sequence.first() {
            // Sanity check
            assert!(matches!(init, QueryState::Init(_)));
            start_unix_ns = Some(init.span().start());

            for state in &query.sequence {
                match state {
                    qa::query::QueryState::Planning(span) => {
                        planning_s = Some(try_to_secs_relative(span.start(), init.span().start())?);
                    }
                    qa::query::QueryState::Executing(span) => {
                        executing_s =
                            Some(try_to_secs_relative(span.start(), init.span().start())?);
                        completed_s = Some(try_to_secs_relative(span.end(), init.span().start())?);
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            id: query.id,
            query_group_id: query.query_group_id,
            instance_name: query.instance_name.clone(),
            start_unix_ns,
            planning_s,
            executing_s,
            completed_s,
        })
    }
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

impl TryFrom<(&qa::worker::Worker, TimeUnixNanoSec)> for Worker {
    type Error = AnalyzerError;

    fn try_from(value: (&qa::worker::Worker, TimeUnixNanoSec)) -> Result<Self, Self::Error> {
        let (worker, _engine_start) = value;
        Ok(Self {
            id: worker.id,
            parent_engine_id: worker.parent_engine_id,
            instance_name: worker.instance_name.clone(),
            start_unix_ns: worker.start_unix_ns,
            end_unix_ns: worker.end_unix_ns,
        })
    }
}

/// An edge between two [`Operator`] [`Port`]s.
#[derive(TS, Debug, Serialize)]
pub struct Edge {
    /// The [`Port`] that produced the data flowing over this edge.
    source: Uuid,
    /// The [`Port`] that consumed the data flowing over this edge.
    target: Uuid,
}

/// A plan for executing a [`Query`].
///
/// The topology of the plan is a Directed Acyclic Graph (DAG).
#[derive(TS, Debug, Serialize)]
pub struct Plan {
    /// The ID of this [`Plan`].
    id: Uuid,
    /// The name of this [`Plan`].
    instance_name: Option<String>,
    /// The ID of the parent [`Plan`], if any.
    parent: Option<Uuid>,
    /// The ID of the [`super::worker::Worker`] that executed this [`Plan`].
    ///
    /// If this level of [`Plan`] was not directly executed by a [`Worker`],
    /// then this is set to None.
    worker_id: Option<Uuid>,
    /// The [`Edge`]s between [`Operator`] [`Port`]s of this [`Plan`].
    edges: Vec<Edge>,
}

impl TryFrom<(&qa::plan::Plan, TimeUnixNanoSec)> for Plan {
    type Error = AnalyzerError;

    fn try_from(value: (&qa::plan::Plan, TimeUnixNanoSec)) -> Result<Self, Self::Error> {
        let (plan, _engine_start) = value;

        let parent = plan.parent.as_ref().map(|p| match p {
            qe::plan::PlanParent::Query(uuid) => *uuid,
            qe::plan::PlanParent::Plan(uuid) => *uuid,
        });

        Ok(Self {
            id: plan.id,
            instance_name: plan.instance_name.clone(),
            parent,
            worker_id: plan.worker_id,
            edges: plan
                .edges
                .iter()
                .map(|e| Edge {
                    source: e.source,
                    target: e.target,
                })
                .collect(),
        })
    }
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
}

impl TryFrom<(&qa::operator::Operator, TimeUnixNanoSec)> for Operator {
    type Error = AnalyzerError;

    fn try_from(value: (&qa::operator::Operator, TimeUnixNanoSec)) -> Result<Self, Self::Error> {
        let (operator, _engine_start) = value;

        let statistics = operator.statistics.as_ref().map(|s| OperatorStatistics {
            custom_statistics: s
                .custom_statistics
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        });

        Ok(Self {
            id: operator.id,
            plan_id: operator.plan_id,
            parent_operator_ids: operator.parent_operator_ids.clone(),
            instance_name: operator.instance_name.clone(),
            operator_type_name: operator.operator_type_name.clone(),
            custom_attributes: operator
                .custom_attributes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            statistics,
        })
    }
}

#[derive(TS, Debug, Serialize)]
pub struct PortStatistics {
    /// Custom statistics
    pub custom_statistics: HashMap<String, Option<Value>>,
}

impl From<&qa::port::PortStatistics> for PortStatistics {
    fn from(value: &qa::port::PortStatistics) -> Self {
        Self {
            custom_statistics: value
                .custom_statistics
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
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

impl TryFrom<(&qa::port::Port, TimeUnixNanoSec)> for Port {
    type Error = AnalyzerError;

    fn try_from(value: (&qa::port::Port, TimeUnixNanoSec)) -> Result<Self, Self::Error> {
        let (port, _engine_start) = value;
        Ok(Self {
            id: port.id,
            operator_id: port.operator_id,
            instance_name: port.instance_name.clone(),
            statistics: port.statistics.as_ref().map(Into::into),
        })
    }
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

impl From<&qa::plan::tree::PlanTree> for PlanTree {
    fn from(tree: &qa::plan::tree::PlanTree) -> Self {
        Self {
            id: tree.id,
            worker: tree.worker,
            children: tree.children.iter().map(Into::into).collect(),
        }
    }
}
