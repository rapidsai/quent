// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Types for the per-operator data-flow categorical timeline.

use std::collections::HashMap;

use quent_time::bin::BinnedSpanSec;
use quent_ui::timeline::categorical::{CategoricalDecl, CategoricalSeries};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

/// A binned data-flow categorical timeline covering every operator of a
/// query. Analyzers without data-flow telemetry return
/// `AnalyzerError::Unsupported` instead (HTTP 501), which the UI treats as
/// "hide the view".
#[derive(TS, Debug, Clone, Serialize)]
pub struct DataFlowTimelineBinned {
    /// The configuration of the binned timeline.
    ///
    /// This may slightly differ from the requested configuration to ensure
    /// bounds are not exceeded and bin sizes are equal.
    pub config: BinnedSpanSec,
    /// Presentation metadata declared by the analyzer.
    pub decl: CategoricalDecl,
    /// Categorical series keyed by operator id.
    pub operators: HashMap<Uuid, CategoricalSeries>,
}
