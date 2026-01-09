use py_rs::PY;
use quent_events::attributes::Attribute;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{EntityRef, span::Span};

#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceTimelineUse {
    pub span: Span,
    pub amounts: Vec<Attribute>,
    pub entity: EntityRef,
}

#[derive(TS, PY, Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResourceTimeline {
    pub span: Span,
    pub uses: Vec<ResourceTimelineUse>,
}
