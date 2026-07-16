// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Requests and responses for distribution timelines: binned timelines of a
//! weighted distribution over (FSM state, application-defined dimension)
//! pairs, for one or more application-declared measures.
//!
//! All semantics are declared by the downstream analyzer: which FSM the states
//! belong to, what the dimension keys mean, and which measures exist. The UI
//! renders the declared names verbatim.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{quantity::CapacityKind, timeline::request::TimelineConfig};

/// Request for a distribution timeline.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct DistributionTimelineRequest<GlobalParams> {
    /// Names of the measures to compute. Empty means all declared measures.
    pub measures: Vec<String>,
    /// The configuration of the window and number of bins.
    pub config: TimelineConfig,
    /// Global application-specific parameters, e.g. filters.
    pub app_params: GlobalParams,
}

/// A measure declared by the downstream analyzer for a distribution timeline,
/// e.g. an entity count or a number of bytes.
#[derive(TS, Debug, Clone, Serialize)]
pub struct MeasureDecl {
    /// Unique name; key into [`DistributionSeries::values`].
    pub name: String,
    /// Human-friendly display name.
    pub display_name: String,
    /// Key into the application's quantity specs map for unit formatting.
    pub quantity: String,
    /// Display semantics of a bin value (Occupancy = time-weighted level).
    pub kind: CapacityKind,
}

/// One key of the application-defined dimension of a distribution timeline.
#[derive(TS, Debug, Clone, Serialize)]
pub struct DimensionKeyDecl {
    /// The key used in [`DistributionSeries::values`].
    pub key: String,
    /// Human-friendly display name.
    pub display_name: String,
}

/// Presentation metadata for a distribution timeline, declared by the
/// downstream analyzer.
///
/// Dimension keys are expected to be a small enumerable set; unbounded key
/// cardinality is a downstream misuse.
#[derive(TS, Debug, Clone, Serialize)]
pub struct DistributionDecl {
    /// The FSM type whose states are distributed. References an entry in the
    /// application's FSM type declarations (e.g. `QueryBundle` fsm_types) for
    /// the state graph, names, and ordering.
    pub entity_type_name: String,
    /// Display name of the dimension, e.g. an application may use a dimension
    /// describing where an entity's data resides.
    pub dimension_name: String,
    /// The dimension keys in stable stacking/legend order.
    pub dimension_keys: Vec<DimensionKeyDecl>,
    /// The measures present in this response.
    pub measures: Vec<MeasureDecl>,
    /// The measure the UI should select by default; must name an entry in
    /// `measures`. `None` means the first declared measure.
    pub default_measure: Option<String>,
}

/// Binned values of one distribution timeline series:
/// measure name -> state name -> dimension key -> one value per time bin.
///
/// Absent inner entries mean all-zero bins.
#[derive(TS, Debug, Clone, Default, Serialize)]
pub struct DistributionSeries {
    pub values: HashMap<String, HashMap<String, HashMap<String, Vec<f64>>>>,
}
