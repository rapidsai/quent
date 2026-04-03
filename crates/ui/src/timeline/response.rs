// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use quent_time::bin::BinnedSpanSec;
use serde::Serialize;
use ts_rs::TS;

use crate::FiniteStateMachine;

#[derive(TS, Debug, Clone, Serialize)]
pub struct ResourceTimelineBinned {
    /// The configuration of the binned timeline.
    pub config: BinnedSpanSec,
    /// Maps a resource capacity name to a vector where each element holds an
    /// aggregated value of a time bin.
    pub capacities_values: HashMap<String, Vec<f64>>,
    /// FSMs that have usage spans exceeding the long_entities_threshold.
    pub long_fsms: Vec<FiniteStateMachine>,
}

#[derive(TS, Debug, Clone, Serialize)]
pub struct ResourceTimelineBinnedByState {
    /// The configuration of the binned timeline.
    pub config: BinnedSpanSec,
    /// Maps a resource capacity name to a map of a state name to a vector where
    /// each element holds an aggregated value of a time bin.
    pub capacities_states_values: HashMap<String, HashMap<String, Vec<f64>>>,
    /// FSMs that have usage spans exceeding the long_entities_threshold.
    pub long_fsms: Vec<FiniteStateMachine>,
}

#[derive(TS, Debug, Clone, Serialize)]
pub enum ResourceTimeline {
    Binned(ResourceTimelineBinned),
    BinnedByState(ResourceTimelineBinnedByState),
}

#[derive(TS, Debug, Clone, Serialize)]
pub struct SingleTimelineResponse {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpanSec,
    /// The data of the response.
    pub data: ResourceTimeline,
}

/// A single entry in a bulk timeline response.
#[derive(TS, Debug, Serialize)]
#[serde(tag = "status")]
pub enum BulkTimelinesResponseEntry {
    #[serde(rename = "ok")]
    Ok {
        /// An informational message about the entry.
        message: String,
        /// The configuration of the binned timeline for this entry.
        config: BinnedSpanSec,
        /// The timeline data for this entry.
        data: ResourceTimeline,
    },
    #[serde(rename = "error")]
    Error {
        /// A message describing the error.
        message: String,
    },
}

/// Response for a bulk timeline request.
#[derive(TS, Debug, Serialize)]
pub struct BulkTimelinesResponse {
    /// The timeline responses, keyed by the same keys as the request entries.
    pub entries: HashMap<String, BulkTimelinesResponseEntry>,
}
