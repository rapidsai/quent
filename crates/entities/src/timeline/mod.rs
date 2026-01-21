use std::collections::HashMap;

use py_rs::PY;
use quent_time::{Span, bin::BinnedSpan};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{EntityRef, resource::CapacityValue};

/// An individual usage of a resource.
#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceTimelineUse {
    /// The span of time in which the resource was utilized.
    pub span: Span,
    /// The amounts of the resource's capacity that was utilized.
    pub amounts: Vec<CapacityValue>,
    /// The entity that utilized the resource.
    pub entity: EntityRef,
}

/// A timeline of individual [`ResourceTimelineUse`]s.
#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceTimeline {
    /// The span of time for which the usages are included.
    pub span: Span,
    /// The uses, arbitrarily ordered.
    pub uses: Vec<ResourceTimelineUse>,
}

#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceTimelineBinned {
    /// The configuration of the binned timeline.
    pub config: BinnedSpan,
    /// Maps a resource capacity name to a vector where each element holds an
    /// aggregated value of a time bin.
    pub capacities_values: HashMap<String, Vec<f64>>,
}

#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceTimelineBinnedByState {
    /// The configuration of the binned timeline.
    pub config: BinnedSpan,
    /// Maps a resource capacity name to a map of a state name to a vector where
    /// each element holds an aggregated value of a time bin.
    pub capacities_states_values: HashMap<String, HashMap<String, Vec<f64>>>,
}
