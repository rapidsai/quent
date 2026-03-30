// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_query_engine_analyzer::ui::UiAnalyzer;

use crate::{analyzer_cache::AnalyzerCache, timeline_cache::TimelineCache};

/// Combined service state for axum handlers.
pub struct ServiceState<A>
where
    A: UiAnalyzer,
{
    pub analyzers: AnalyzerCache<A>,
    pub timelines: TimelineCache,
}

impl<A> Clone for ServiceState<A>
where
    A: UiAnalyzer,
{
    fn clone(&self) -> Self {
        Self {
            analyzers: self.analyzers.clone(),
            timelines: self.timelines.clone(),
        }
    }
}
