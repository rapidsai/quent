// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Types for the per-operator data-flow distribution timeline.

use std::collections::HashMap;

use quent_time::bin::BinnedSpanSec;
use quent_ui::timeline::distribution::{DistributionDecl, DistributionSeries};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

/// A binned data-flow distribution timeline covering every operator of a
/// query.
#[derive(TS, Debug, Clone, Serialize)]
pub struct DataFlowTimelineBinned {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpanSec,
    /// Presentation metadata declared by the analyzer.
    pub decl: DistributionDecl,
    /// Distribution series keyed by operator id.
    pub operators: HashMap<Uuid, DistributionSeries>,
}

/// Response for a data-flow distribution timeline request.
#[derive(TS, Debug, Clone, Serialize)]
pub enum DataFlowTimelineResponse {
    /// This analyzer does not provide data-flow distributions; the UI hides
    /// the corresponding view.
    Unsupported,
    Binned(DataFlowTimelineBinned),
}
