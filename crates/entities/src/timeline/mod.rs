use std::collections::HashMap;

use quent_time::bin::BinnedSpanSec;
use serde::Serialize;
use ts_rs::TS;

#[derive(TS, Clone, Debug, Serialize)]
pub struct ResourceTimelineBinned {
    /// The configuration of the binned timeline.
    pub config: BinnedSpanSec,
    /// Maps a resource capacity name to a vector where each element holds an
    /// aggregated value of a time bin.
    pub capacities_values: HashMap<String, Vec<f64>>,
}

#[derive(TS, Clone, Debug, Serialize)]
pub struct ResourceTimelineBinnedByState {
    /// The configuration of the binned timeline.
    pub config: BinnedSpanSec,
    /// Maps a resource capacity name to a map of a state name to a vector where
    /// each element holds an aggregated value of a time bin.
    pub capacities_states_values: HashMap<String, HashMap<String, Vec<f64>>>,
}
