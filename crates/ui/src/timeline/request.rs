// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, num::NonZero};

use quent_time::{
    TimeError, TimeSec, TimeUnixNanoSec, bin::BinnedSpan, span::SpanUnixNanoSec, to_nanosecs,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// Configuration of the window and number of bins of a timeline.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct TimelineConfig {
    /// The number of bins for binned timelines.
    pub num_bins: u16,
    /// The start time of the time window applied to all timelines in seconds.
    pub start: f64,
    /// The end time of the time window applied to all timelines in seconds.
    pub end: f64,
}

impl TimelineConfig {
    pub fn try_into_binned_span(
        self,
        epoch: TimeUnixNanoSec,
    ) -> std::result::Result<BinnedSpan, TimeError> {
        BinnedSpan::try_new(
            SpanUnixNanoSec::try_new(
                epoch + to_nanosecs(self.start),
                epoch + to_nanosecs(self.end),
            )?,
            NonZero::try_from(self.num_bins as u64).map_err(|e| {
                TimeError::InvalidArgument(format!("number of bins must be > 0: {e}"))
            })?,
        )
    }
}

#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct EntityFilter {
    /// If set, only include utilizations from entities with this type name.
    ///
    /// If this entity is an FSM, then the timeline will aggregate usages into
    /// bins for each state separately.
    pub entity_type_name: Option<String>,
    // TODO(johanpel): instance name
}

/// Parameters for requesting a resource timeline.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTimelineRequest<TimelineParams> {
    /// The ID of the resource
    pub resource_id: Uuid,
    /// If set, fully include entities that have usages exceeding this amount of time.
    pub long_entities_threshold_s: Option<TimeSec>,
    /// Entity filters.
    pub entity_filter: EntityFilter,
    /// Application-specific request parameters, e.g. for filtering.
    pub application: TimelineParams,
    /// The configuration of the window and number of bins.
    pub config: TimelineConfig,
}

/// Parameters for requesting a resource group timeline.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGroupTimelineRequest<TimelineParams> {
    /// The ID of the resource group
    pub resource_group_id: Uuid,
    /// The type name of the leaf resources for which to produce the timeline
    /// for this group.
    pub resource_type_name: String,
    /// If set, fully include entities that have usages exceeding this amount of
    /// time in seconds.
    pub long_entities_threshold_s: Option<TimeSec>,
    /// Entity filters.
    pub entity_filter: EntityFilter,
    /// Application-specific request parameters, e.g. for filtering.
    pub app_params: TimelineParams,
    /// The configuration of the window and number of bins.
    pub config: TimelineConfig,
}

/// Timeline request parameters unrelated to timing or binning.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub enum TimelineRequest<TimelineParams> {
    /// Request for a resource timeline.
    Resource(ResourceTimelineRequest<TimelineParams>),
    /// Request for a resource group timeline.
    ResourceGroup(ResourceGroupTimelineRequest<TimelineParams>),
}

impl<T> TimelineRequest<T> {
    pub fn config(&self) -> &TimelineConfig {
        match self {
            Self::Resource(r) => &r.config,
            Self::ResourceGroup(rg) => &rg.config,
        }
    }

    pub fn with_config(self, config: TimelineConfig) -> Self {
        match self {
            Self::Resource(r) => Self::Resource(ResourceTimelineRequest { config, ..r }),
            Self::ResourceGroup(rg) => {
                Self::ResourceGroup(ResourceGroupTimelineRequest { config, ..rg })
            }
        }
    }
}

/// Request for a single timeline.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct SingleTimelineRequest<GlobalParams, TimelineParams> {
    /// The timeline requested.
    pub entry: TimelineRequest<TimelineParams>,
    /// Global application-specific parameters, e.g. filters.
    pub app_params: GlobalParams,
}

/// Request for a bulk of timelines.
#[derive(TS, Debug, Deserialize)]
pub struct BulkTimelineRequest<GlobalParams, TimelineParams> {
    /// The list of timelines requested.
    pub entries: HashMap<String, TimelineRequest<TimelineParams>>,
    /// Global application-specific parameters, e.g. filters.
    pub app_params: GlobalParams,
}
