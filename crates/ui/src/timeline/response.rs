use std::collections::HashMap;

use quent_time::bin::BinnedSpanSec;
use serde::Serialize;
use ts_rs::TS;

use crate::FiniteStateMachine;

#[derive(TS, Debug, Serialize)]
pub struct ResourceTimelineBinned {
    /// Maps a resource capacity name to a vector where each element holds an
    /// aggregated value of a time bin.
    pub capacities_values: HashMap<String, Vec<f64>>,
    /// FSMs that have usage spans exceeding the long_entities_threshold.
    pub long_fsms: Vec<FiniteStateMachine>,
}

#[derive(TS, Debug, Serialize)]
pub struct ResourceTimelineBinnedByState {
    /// Maps a resource capacity name to a map of a state name to a vector where
    /// each element holds an aggregated value of a time bin.
    pub capacities_states_values: HashMap<String, HashMap<String, Vec<f64>>>,
    /// FSMs that have usage spans exceeding the long_entities_threshold.
    pub long_fsms: Vec<FiniteStateMachine>,
}

#[derive(TS, Debug, Serialize)]
pub enum ResourceTimeline {
    Binned(ResourceTimelineBinned),
    BinnedByState(ResourceTimelineBinnedByState),
}

#[derive(TS, Debug, Serialize)]
pub struct SingleTimelineResponse {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpanSec,
    /// The data of the response.
    pub data: ResourceTimeline,
}

#[derive(TS, Debug, Serialize)]
#[serde(tag = "status")]
pub enum BulkTimelinesResponseEntry {
    #[serde(rename = "ok")]
    Ok {
        message: String,
        data: ResourceTimeline,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(TS, Debug, Serialize)]
pub struct BulkTimelinesResponse {
    pub config: BinnedSpanSec,
    pub entries: HashMap<String, BulkTimelinesResponseEntry>,
}
