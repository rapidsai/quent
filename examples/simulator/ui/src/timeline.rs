use std::collections::HashMap;

use quent_time::{TimeSec, bin::BinnedSpanSec};
use quent_ui::FiniteStateMachine;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(TS, Debug, Serialize)]
pub struct ResourceTimelineBinned {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpanSec,
    /// Maps a resource capacity name to a vector where each element holds an
    /// aggregated value of a time bin.
    pub capacities_values: HashMap<String, Vec<f64>>,
    /// FSMs that have usage spans exceeding the long_entities_threshold.
    pub long_fsms: Vec<FiniteStateMachine>,
}

#[derive(TS, Debug, Serialize)]
pub struct ResourceTimelineBinnedByState {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpanSec,
    /// Maps a resource capacity name to a map of a state name to a vector where
    /// each element holds an aggregated value of a time bin.
    pub capacities_states_values: HashMap<String, HashMap<String, Vec<f64>>>,
    /// FSMs that have usage spans exceeding the long_entities_threshold.
    pub long_fsms: Vec<FiniteStateMachine>,
}

#[derive(TS, Debug, Serialize)]
pub enum TimelineResponse {
    Binned(ResourceTimelineBinned),
    BinnedByState(ResourceTimelineBinnedByState),
}

#[derive(TS, Debug, Deserialize)]
pub struct ResourceTimelineUrlQueryParams {
    /// The number of bins.
    ///
    /// u16::MAX is large enough when bins are plotted as single pixel wide
    /// bars, even for insane screen resolutions.
    pub num_bins: u16,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,

    /// If set, only include utilizations from FSMs with this type name, and
    /// aggregate for each state separately.
    ///
    /// Can be set for both resource and resource group timelines.
    pub fsm_type_name: Option<String>,

    /// Sets the resource type for which to provide an aggregated timeline.
    ///
    /// This is required for resource group routes, and is ignored for
    /// individual resource timeline routes.
    pub resource_type_name: Option<String>,

    /// Filter the usages of the resource (group) on this operator ID.
    //
    // TODO(johanpel): this will only work for FSMs directly referencing this operator.
    pub operator_id: Option<Uuid>,
    /// If set, fully include entities that have usages exceeding this amount of
    /// time in seconds.
    pub long_entities_threshold_s: Option<TimeSec>,
}

/// Parameters for requesting a resource timeline.
#[derive(TS, Debug, Deserialize)]
pub struct ResourceTimelineRequestParams {
    /// The type name of the FSM for which to produce the resource utilization timeline.
    ///
    /// If set, only include utilizations from FSMs with this type name, and
    /// aggregate for each state separately.
    pub fsm_type_name: Option<String>,

    /// Filter the usages of the resource on this operator ID.
    //
    // TODO(johanpel): this will only work for FSMs directly referencing this operator.
    pub operator_id: Option<Uuid>,
    /// If set, fully include entities that have usages exceeding this amount of time.
    pub long_entities_threshold_s: Option<TimeSec>,
}

/// Parameters for requesting a resource group timeline.
#[derive(TS, Debug, Deserialize)]
pub struct ResourceGroupTimelineRequestParams {
    /// The type name of the FSM for which to produce the resource utilization
    /// timeline.
    ///
    /// If set, only include utilizations from FSMs with this type name, and
    /// aggregate for each state separately.
    pub fsm_type_name: Option<String>,
    /// The type name of the leaf resources for which to produce the timeline
    /// for this group.
    pub resource_type_name: String,
    /// Filter the usages of the leaf resources by this operator ID.
    //
    // TODO(johanpel): this will only work for FSMs directly referencing this operator.
    pub operator_id: Option<Uuid>,
    /// If set, fully include entities that have usages exceeding this amount of
    /// time in seconds.
    pub long_entities_threshold_s: Option<TimeSec>,
}

#[derive(TS, Debug, Deserialize)]
#[serde(untagged)]
pub enum BulkTimelineRequestParams {
    ResourceGroup(ResourceGroupTimelineRequestParams),
    Resource(ResourceTimelineRequestParams),
}

/// A request for timelines in bulk.
#[derive(TS, Debug, Deserialize)]
pub struct BulkTimelinesRequest {
    pub num_bins: u16,
    pub start: f64,
    pub end: f64,
    /// A map of resource_(group)_id to a request.
    pub entries: HashMap<Uuid, BulkTimelineRequestParams>,
}

/// The data of a single resource timeline within a bulk response.
/// Config is omitted here because it is shared across all resources.
#[derive(TS, Debug, Serialize)]
#[serde(tag = "type")]
pub enum BulkTimelineData {
    Binned {
        capacities_values: HashMap<String, Vec<f64>>,
        long_fsms: Vec<FiniteStateMachine>,
    },
    BinnedByState {
        capacities_states_values: HashMap<String, HashMap<String, Vec<f64>>>,
        long_fsms: Vec<FiniteStateMachine>,
    },
}

#[derive(TS, Debug, Serialize)]
#[serde(tag = "status")]
pub enum BulkTimelineResponseEntry {
    #[serde(rename = "ok")]
    Ok {
        message: String,
        data: BulkTimelineData,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(TS, Debug, Serialize)]
pub struct BulkTimelinesResponse {
    pub config: BinnedSpanSec,
    pub resources: HashMap<String, BulkTimelineResponseEntry>,
}
