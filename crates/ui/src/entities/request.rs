// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use quent_analyzer::{AnalyzerResult, Model, resource::tree::ResourceTreeNode};
use quent_time::{TimeError, TimeSec, TimeUnixNanoSec, span::SpanUnixNanoSec, to_nanosecs};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::paginate::PageParams;

/// Restricts returned entities to appear in a certain scope.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub enum EntityScope {
    /// Only return entities that use this resource.
    Resource { resource_id: Uuid },
    /// Only return entities that use any resource within this group.
    ResourceGroup {
        resource_group_id: Uuid,
        resource_type_name: String,
    },
}

impl EntityScope {
    /// Resolve the scope to the resource IDs it covers: the single resource, or
    /// the leaf resources of the group that have the requested type.
    pub fn resolve(&self, model: &impl Model) -> AnalyzerResult<HashSet<Uuid>> {
        match self {
            EntityScope::Resource { resource_id } => {
                model.resource(*resource_id)?;
                Ok([*resource_id].into_iter().collect())
            }
            EntityScope::ResourceGroup {
                resource_group_id,
                resource_type_name,
            } => {
                let tree = ResourceTreeNode::try_new(model, *resource_group_id)?;
                Ok(tree
                    .iter_leaf_ids()
                    .filter(|&id| {
                        model
                            .resource(id)
                            .is_ok_and(|r| r.type_name() == resource_type_name)
                    })
                    .collect())
            }
        }
    }
}

/// A time window, resolved against an epoch supplied at conversion.
#[derive(TS, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimeWindow {
    /// The start time of the window in seconds.
    pub start: TimeSec,
    /// The end time of the window in seconds.
    pub end: TimeSec,
}

impl TimeWindow {
    /// Resolve to an absolute span by offsetting from `epoch`.
    pub fn try_into_span(self, epoch: TimeUnixNanoSec) -> Result<SpanUnixNanoSec, TimeError> {
        SpanUnixNanoSec::try_new(
            epoch + to_nanosecs(self.start),
            epoch + to_nanosecs(self.end),
        )
    }
}

/// Entity filters.
///
/// Every field that is set must match, `None` fields do not filter.
#[derive(TS, Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityListFilter {
    /// Restrict resulting entities to be in this scope.
    pub scope: Option<EntityScope>,
    /// Restrict resulting entities to be of this type.
    pub entity_type_name: Option<String>,
    /// Keep only entities with resource usages longer than this threshold.
    ///
    /// N.B. only Fsm-type entities can have usages.
    pub min_usage_s: Option<TimeSec>,
}

/// The key entities are sorted by.
#[derive(TS, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EntitySortKey {
    /// The longest single resource-usage span within the window.
    UsageDuration,
}

/// The direction to sort in.
#[derive(TS, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortDir {
    /// Sort in ascending order.
    Asc,
    /// Sort in descending order.
    Desc,
}

/// Sorting parameters.
#[derive(TS, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sort {
    /// The key to sort on.
    pub key: EntitySortKey,
    /// The direction to sort in.
    pub dir: SortDir,
}

/// A single entity-list query.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct EntityListEntry<EntryParams> {
    /// The window of time entities must fall in.
    pub window: TimeWindow,
    /// Filter parameters.
    pub filter: EntityListFilter,
    /// Sort parameters.
    pub sort: Sort,
    /// Pagination parameters.
    ///
    /// When this is not set, return the full list. Depending on the dataset
    /// size and other parameters, this may result in a large volume of data and
    /// should be used with care.
    pub page: Option<PageParams>,
    /// Per-query application-specific parameters.
    pub application: EntryParams,
}

/// Parameters for listing entities.
#[derive(TS, Debug, Clone, Serialize, Deserialize)]
pub struct EntityListRequest<GlobalParams, EntryParams> {
    pub entry: EntityListEntry<EntryParams>,
    /// Global application parameters shared by the query.
    pub app_params: GlobalParams,
}
