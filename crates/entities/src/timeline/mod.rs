use std::collections::HashMap;

use py_rs::PY;
use quent_attributes::Attribute;
use quent_time::{Span, bin::BinnedSpan};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::EntityRef;

#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceTimelineUse {
    pub span: Span,
    pub amounts: Vec<Attribute>,
    pub entity: EntityRef,
}

#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceTimeline {
    pub span: Span,
    pub uses: Vec<ResourceTimelineUse>,
}

#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceTimelineBinned {
    pub config: BinnedSpan,
    pub capacity_values: HashMap<String, Vec<f64>>,
}
