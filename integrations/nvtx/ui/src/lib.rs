// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! UI-facing NVTX contracts and model-to-viewport conversion.
//!
//! The exchange types deliberately contain presentation semantics, not capture
//! internals: domain/category selection, lane identities, nesting depth,
//! clipped display bounds, and viewport-scoped statistics are all resolved here.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

use nvtx_analyzer::{NvtxColor, NvtxModel, NvtxSpan, SpanId, SpanKind};
use quent_time::{TimeUnixNanoSec, to_nanosecs, to_secs, to_secs_relative};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Stable, output-only metadata for one NVTX stream.
#[derive(TS, Debug, Clone, PartialEq, Serialize)]
pub struct NvtxCatalog {
    /// Server-side absolute origin used to produce every relative-second field.
    #[serde(skip_serializing)]
    #[ts(skip)]
    query_start: TimeUnixNanoSec,
    /// Trace start in seconds relative to the query start.
    pub trace_start: f64,
    /// Trace end in seconds relative to the query start.
    pub trace_end: f64,
    pub domains: Vec<NvtxCatalogDomain>,
    pub anomalies: NvtxCatalogAnomalies,
}

/// Reconstruction events that could not be represented faithfully in the model.
#[derive(TS, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvtxCatalogAnomalies {
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub orphan_range_ends: u64,
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub orphan_range_pops: u64,
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub orphan_resource_destroys: u64,
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub reused_range_ids: u64,
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub reused_resource_handles: u64,
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub total: u64,
    pub is_faithful: bool,
}

/// Selectable metadata for one domain.
#[derive(TS, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvtxCatalogDomain {
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    pub name: String,
    pub color: String,
    pub threads: Vec<NvtxCatalogThread>,
    pub categories: Vec<NvtxCatalogCategory>,
    pub has_uncategorized: bool,
}

#[derive(TS, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvtxCatalogThread {
    pub thread_id: u32,
    pub name: String,
}

#[derive(TS, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvtxCatalogCategory {
    pub category_id: u32,
    pub name: String,
}

/// Inclusive viewport bounds in seconds relative to the query start.
#[derive(TS, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NvtxViewportWindow {
    pub start: f64,
    pub end: f64,
}

/// One domain's selected categories.
#[derive(TS, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NvtxDomainSelection {
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    pub category_ids: Vec<u32>,
    pub include_uncategorized: bool,
}

/// Request for one atomically-scoped set of lanes and statistics.
#[derive(TS, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvtxViewportRequest {
    pub viewport: NvtxViewportWindow,
    pub selections: Vec<NvtxDomainSelection>,
}

/// UI-ready NVTX content for one viewport and selection.
#[derive(TS, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvtxViewportResponse {
    pub viewport: NvtxViewportWindow,
    pub domains: Vec<NvtxDomainLaneGroup>,
    pub statistics: Vec<NvtxRangeStatistics>,
}

#[derive(TS, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvtxDomainLaneGroup {
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    pub name: String,
    pub color: String,
    pub lanes: Vec<NvtxLane>,
}

/// A truthful NVTX lane identity. Thread depth rows are explicit rather than
/// reconstructed by TypeScript.
#[derive(TS, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NvtxLaneIdentity {
    Thread { thread_id: u32, depth: u32 },
    Process,
    Marks,
}

#[derive(TS, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvtxLane {
    pub id: String,
    pub label: String,
    pub identity: NvtxLaneIdentity,
    pub ranges: Vec<NvtxRangeItem>,
    pub marks: Vec<NvtxMarkItem>,
}

#[derive(TS, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NvtxRangeKind {
    PushPop,
    StartEnd,
}

#[derive(TS, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvtxRangeItem {
    pub message: String,
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    pub domain_name: String,
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub color: String,
    pub kind: NvtxRangeKind,
    pub thread_id: Option<u32>,
    pub thread_name: Option<String>,
    /// The actual captured start in seconds relative to the query start.
    pub observed_start: f64,
    /// The actual captured close in relative seconds; absent for an incomplete range.
    pub observed_end: Option<f64>,
    /// Relative-second bounds clipped to the requested viewport for rendering.
    pub display_start: f64,
    pub display_end: f64,
    /// The completed range duration in seconds.
    pub observed_duration: Option<f64>,
    pub incomplete: bool,
}

#[derive(TS, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvtxMarkItem {
    pub message: String,
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    pub domain_name: String,
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub color: String,
    /// Mark timestamp in seconds relative to the query start.
    pub timestamp: f64,
}

/// Statistics over exactly the filtered, intersecting range population.
/// Closed durations are clipped to the visible window; incomplete ranges are
/// counted but never assigned an inferred duration. All duration fields are in
/// seconds.
#[derive(TS, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvtxRangeStatistics {
    pub message: String,
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    pub domain_name: String,
    pub category_id: Option<u32>,
    pub category_name: Option<String>,
    pub count: u64,
    pub observed_count: u64,
    pub total_duration: f64,
    pub avg_duration: f64,
    pub min_duration: Option<f64>,
    pub max_duration: Option<f64>,
    pub saturated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NvtxViewportError {
    InvalidWindow,
    EmptySelection { domain_id: u64 },
    DuplicateDomain { domain_id: u64 },
    UnknownDomain { domain_id: u64 },
    UnknownCategory { domain_id: u64, category_id: u32 },
    UncategorizedUnavailable { domain_id: u64 },
}

impl fmt::Display for NvtxViewportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWindow => write!(f, "viewport bounds must be finite and ordered"),
            Self::EmptySelection { domain_id } => {
                write!(f, "domain {domain_id} selects no categories")
            }
            Self::DuplicateDomain { domain_id } => {
                write!(f, "domain {domain_id} appears more than once")
            }
            Self::UnknownDomain { domain_id } => write!(f, "unknown domain {domain_id}"),
            Self::UnknownCategory {
                domain_id,
                category_id,
            } => write!(f, "unknown category {category_id} in domain {domain_id}"),
            Self::UncategorizedUnavailable { domain_id } => {
                write!(f, "domain {domain_id} has no uncategorized items")
            }
        }
    }
}

impl Error for NvtxViewportError {}

#[derive(Default)]
struct CatalogDomainMetadata {
    thread_ids: BTreeSet<u32>,
    has_uncategorized: bool,
}

impl NvtxCatalog {
    /// Build catalog metadata with times relative to `query_start`.
    pub fn from_model(model: &NvtxModel, query_start: TimeUnixNanoSec) -> Self {
        let thread_names: HashMap<u32, &str> = model
            .threads()
            .iter()
            .map(|thread| (thread.thread_id, thread.name.as_str()))
            .collect();

        let mut metadata_by_domain = HashMap::<u64, CatalogDomainMetadata>::new();
        for span in model.spans() {
            let metadata = metadata_by_domain.entry(span.domain).or_default();
            if let SpanKind::PushPop { thread_id, .. } = span.kind {
                metadata.thread_ids.insert(thread_id);
            }
            if is_range(span) && span.category.is_none() {
                metadata.has_uncategorized = true;
            }
        }
        for mark in model.marks() {
            if mark.category.is_none() {
                metadata_by_domain
                    .entry(mark.domain)
                    .or_default()
                    .has_uncategorized = true;
            }
        }

        let mut categories_by_domain = HashMap::<u64, Vec<NvtxCatalogCategory>>::new();
        for category in model.categories() {
            categories_by_domain
                .entry(category.domain)
                .or_default()
                .push(NvtxCatalogCategory {
                    category_id: category.category,
                    name: category.name.clone(),
                });
        }

        let mut domains = model
            .domains()
            .iter()
            .map(|domain| {
                let metadata = metadata_by_domain
                    .remove(&domain.domain)
                    .unwrap_or_default();

                let mut threads: Vec<_> = metadata
                    .thread_ids
                    .into_iter()
                    .map(|thread_id| NvtxCatalogThread {
                        thread_id,
                        name: thread_names.get(&thread_id).map_or_else(
                            || model.thread_name(thread_id),
                            |name| (*name).to_owned(),
                        ),
                    })
                    .collect();
                threads.sort_by(|left, right| {
                    left.name
                        .cmp(&right.name)
                        .then(left.thread_id.cmp(&right.thread_id))
                });

                let mut categories = categories_by_domain
                    .remove(&domain.domain)
                    .unwrap_or_default();
                categories.sort_by(|left, right| {
                    left.name
                        .cmp(&right.name)
                        .then(left.category_id.cmp(&right.category_id))
                });

                NvtxCatalogDomain {
                    domain_id: domain.domain,
                    name: domain.name.clone(),
                    color: fallback_color(domain.domain).to_owned(),
                    threads,
                    categories,
                    has_uncategorized: metadata.has_uncategorized,
                }
            })
            .collect::<Vec<_>>();

        domains.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.domain_id.cmp(&right.domain_id))
        });

        let anomalies = model.anomalies();
        Self {
            query_start,
            trace_start: to_secs_relative(model.trace_start(), query_start),
            trace_end: to_secs_relative(model.trace_end(), query_start),
            domains,
            anomalies: NvtxCatalogAnomalies {
                orphan_range_ends: anomalies.orphan_range_ends,
                orphan_range_pops: anomalies.orphan_range_pops,
                orphan_resource_destroys: anomalies.orphan_resource_destroys,
                reused_range_ids: anomalies.reused_range_ids,
                reused_resource_handles: anomalies.reused_resource_handles,
                total: anomalies.total(),
                is_faithful: anomalies.is_faithful(),
            },
        }
    }

    /// The explicit initial UI state: every catalog option selected.
    pub fn select_all(&self) -> Vec<NvtxDomainSelection> {
        let mut selections: Vec<_> = self
            .domains
            .iter()
            .filter_map(|domain| {
                let category_ids = domain
                    .categories
                    .iter()
                    .map(|category| category.category_id)
                    .collect::<Vec<_>>();
                (!category_ids.is_empty() || domain.has_uncategorized).then_some(
                    NvtxDomainSelection {
                        domain_id: domain.domain_id,
                        category_ids,
                        include_uncategorized: domain.has_uncategorized,
                    },
                )
            })
            .collect();
        selections.sort_by_key(|selection| selection.domain_id);
        selections
    }

    /// Validate and canonicalize a request.
    pub fn canonicalize_request(
        &self,
        mut request: NvtxViewportRequest,
    ) -> Result<NvtxViewportRequest, NvtxViewportError> {
        if !request.viewport.start.is_finite()
            || !request.viewport.end.is_finite()
            || request.viewport.start > request.viewport.end
        {
            return Err(NvtxViewportError::InvalidWindow);
        }

        let catalog: HashMap<_, _> = self
            .domains
            .iter()
            .map(|domain| (domain.domain_id, domain))
            .collect();
        let mut seen = BTreeSet::new();
        for selection in &mut request.selections {
            if !seen.insert(selection.domain_id) {
                return Err(NvtxViewportError::DuplicateDomain {
                    domain_id: selection.domain_id,
                });
            }
            if selection.category_ids.is_empty() && !selection.include_uncategorized {
                return Err(NvtxViewportError::EmptySelection {
                    domain_id: selection.domain_id,
                });
            }
            let domain =
                catalog
                    .get(&selection.domain_id)
                    .ok_or(NvtxViewportError::UnknownDomain {
                        domain_id: selection.domain_id,
                    })?;
            selection.category_ids.sort_unstable();
            selection.category_ids.dedup();
            for category_id in &selection.category_ids {
                if !domain
                    .categories
                    .iter()
                    .any(|category| category.category_id == *category_id)
                {
                    return Err(NvtxViewportError::UnknownCategory {
                        domain_id: selection.domain_id,
                        category_id: *category_id,
                    });
                }
            }
            if selection.include_uncategorized && !domain.has_uncategorized {
                return Err(NvtxViewportError::UncategorizedUnavailable {
                    domain_id: selection.domain_id,
                });
            }
        }
        request
            .selections
            .sort_by_key(|selection| selection.domain_id);
        Ok(request)
    }
}

impl NvtxViewportResponse {
    /// Convert a viewport with all public times relative to `query_start`.
    pub fn from_model(
        model: &NvtxModel,
        query_start: TimeUnixNanoSec,
        request: NvtxViewportRequest,
    ) -> Result<Self, NvtxViewportError> {
        let catalog = NvtxCatalog::from_model(model, query_start);
        Self::from_model_with_catalog(model, &catalog, request)
    }

    /// Convert a viewport using catalog metadata cached for this model and time origin.
    pub fn from_model_with_catalog(
        model: &NvtxModel,
        catalog: &NvtxCatalog,
        request: NvtxViewportRequest,
    ) -> Result<Self, NvtxViewportError> {
        let request = catalog.canonicalize_request(request)?;
        let viewport = absolute_viewport(request.viewport, catalog.query_start)
            .ok_or(NvtxViewportError::InvalidWindow)?;
        let selections: HashMap<_, _> = request
            .selections
            .iter()
            .map(|selection| (selection.domain_id, selection))
            .collect();
        let domains_by_id: HashMap<_, _> = catalog
            .domains
            .iter()
            .map(|domain| (domain.domain_id, domain))
            .collect();
        let depths = span_depths(model);
        let mut statistics = BTreeMap::<StatsGroupKey, StatisticsAccumulator>::new();
        let mut items_by_domain = HashMap::<u64, DomainViewportItems>::new();

        for (index, span) in model.spans().iter().enumerate() {
            let Some(selection) = selections.get(&span.domain) else {
                continue;
            };
            if !is_range(span)
                || !selected(selection, span.category)
                || !intersects(span.start, span.end.unwrap_or(model.trace_end()), viewport)
            {
                continue;
            }

            let domain = domains_by_id
                .get(&span.domain)
                .expect("validated selections only reference catalog domains");
            let Some(item) = range_item(model, domain, span, catalog.query_start, viewport) else {
                continue;
            };
            statistics
                .entry(StatsGroupKey {
                    domain_id: span.domain,
                    category_id: span.category,
                    message: span.name.clone(),
                })
                .or_default()
                .accumulate(span, viewport);
            let domain_items = items_by_domain.entry(span.domain).or_default();
            match span.kind {
                SpanKind::PushPop { thread_id, .. } => {
                    domain_items
                        .thread_lanes
                        .entry((thread_id, depths[index]))
                        .or_default()
                        .push(item);
                }
                SpanKind::StartEnd => domain_items.process_ranges.push(item),
                SpanKind::Resource { .. } => continue,
            }
        }

        for mark in model.marks() {
            let Some(selection) = selections.get(&mark.domain) else {
                continue;
            };
            if !selected(selection, mark.category)
                || mark.timestamp < viewport.start
                || mark.timestamp > viewport.end
            {
                continue;
            }
            let domain = domains_by_id
                .get(&mark.domain)
                .expect("validated selections only reference catalog domains");
            items_by_domain
                .entry(mark.domain)
                .or_default()
                .marks
                .push(NvtxMarkItem {
                    message: mark.name.clone(),
                    domain_id: domain.domain_id,
                    domain_name: domain.name.clone(),
                    category_id: mark.category,
                    category_name: mark
                        .category
                        .and_then(|id| model.category_name(domain.domain_id, id)),
                    color: display_color(mark.color, domain.domain_id),
                    timestamp: to_secs_relative(mark.timestamp, catalog.query_start),
                });
        }

        let mut domains = Vec::new();
        for domain in &catalog.domains {
            let Some(mut domain_items) = items_by_domain.remove(&domain.domain_id) else {
                continue;
            };
            let mut marks = domain_items.marks;
            marks.sort_by(|left, right| {
                left.timestamp
                    .total_cmp(&right.timestamp)
                    .then(left.message.cmp(&right.message))
            });

            let thread_order: HashMap<_, _> = domain
                .threads
                .iter()
                .enumerate()
                .map(|(index, thread)| (thread.thread_id, index))
                .collect();
            let mut lane_entries: Vec<_> = domain_items.thread_lanes.into_iter().collect();
            lane_entries.sort_by(
                |((left_thread, left_depth), _), ((right_thread, right_depth), _)| {
                    thread_order
                        .get(left_thread)
                        .cmp(&thread_order.get(right_thread))
                        .then(left_depth.cmp(right_depth))
                },
            );

            let mut lanes = lane_entries
                .into_iter()
                .map(|((thread_id, depth), mut ranges)| {
                    sort_ranges(&mut ranges);
                    let thread_name = model.thread_name(thread_id);
                    NvtxLane {
                        id: format!("nvtx:{}:thread:{thread_id}:depth:{depth}", domain.domain_id),
                        label: if depth == 0 {
                            thread_name
                        } else {
                            format!("{thread_name} · depth {depth}")
                        },
                        identity: NvtxLaneIdentity::Thread { thread_id, depth },
                        ranges,
                        marks: Vec::new(),
                    }
                })
                .collect::<Vec<_>>();

            if !domain_items.process_ranges.is_empty() {
                sort_ranges(&mut domain_items.process_ranges);
                lanes.push(NvtxLane {
                    id: format!("nvtx:{}:process", domain.domain_id),
                    label: "Process ranges".to_owned(),
                    identity: NvtxLaneIdentity::Process,
                    ranges: domain_items.process_ranges,
                    marks: Vec::new(),
                });
            }
            if !marks.is_empty() {
                lanes.push(NvtxLane {
                    id: format!("nvtx:{}:marks", domain.domain_id),
                    label: "Marks".to_owned(),
                    identity: NvtxLaneIdentity::Marks,
                    ranges: Vec::new(),
                    marks,
                });
            }
            if !lanes.is_empty() {
                domains.push(NvtxDomainLaneGroup {
                    domain_id: domain.domain_id,
                    name: domain.name.clone(),
                    color: domain.color.clone(),
                    lanes,
                });
            }
        }

        let domain_order: HashMap<_, _> = catalog
            .domains
            .iter()
            .enumerate()
            .map(|(index, domain)| (domain.domain_id, index))
            .collect();
        let mut statistics = statistics
            .into_iter()
            .map(|(key, accumulator)| {
                let domain = domains_by_id
                    .get(&key.domain_id)
                    .expect("statistics only include catalog domains");
                accumulator.finish(&key, domain, model)
            })
            .collect::<Vec<_>>();
        statistics.sort_by(|left, right| {
            domain_order
                .get(&left.domain_id)
                .cmp(&domain_order.get(&right.domain_id))
                .then(left.category_name.cmp(&right.category_name))
                .then(left.category_id.cmp(&right.category_id))
                .then(left.message.cmp(&right.message))
        });

        Ok(Self {
            viewport: request.viewport,
            domains,
            statistics,
        })
    }
}

#[derive(Default)]
struct DomainViewportItems {
    thread_lanes: BTreeMap<(u32, u32), Vec<NvtxRangeItem>>,
    process_ranges: Vec<NvtxRangeItem>,
    marks: Vec<NvtxMarkItem>,
}

fn is_range(span: &NvtxSpan) -> bool {
    matches!(span.kind, SpanKind::PushPop { .. } | SpanKind::StartEnd)
}

/// Reports whether `category` is selected.
///
/// `selection.category_ids` must be sorted and deduplicated before calling;
/// [`NvtxCatalog::canonicalize_request`] establishes this invariant.
fn selected(selection: &NvtxDomainSelection, category: Option<u32>) -> bool {
    match category {
        Some(id) => selection.category_ids.binary_search(&id).is_ok(),
        None => selection.include_uncategorized,
    }
}

#[derive(Debug, Clone, Copy)]
struct AbsoluteViewport {
    start: u64,
    end: u64,
}

fn absolute_viewport(viewport: NvtxViewportWindow, epoch: u64) -> Option<AbsoluteViewport> {
    let start = absolute_timestamp(viewport.start, epoch)?;
    let end = absolute_timestamp(viewport.end, epoch)?;
    Some(AbsoluteViewport { start, end })
}

fn absolute_timestamp(relative_seconds: f64, epoch: u64) -> Option<u64> {
    let nanoseconds = to_nanosecs(relative_seconds.abs());
    if relative_seconds.is_sign_negative() {
        epoch.checked_sub(nanoseconds)
    } else {
        epoch.checked_add(nanoseconds)
    }
}

fn intersects(start: u64, effective_end: u64, viewport: AbsoluteViewport) -> bool {
    start <= viewport.end && effective_end >= viewport.start
}

fn span_depths(model: &NvtxModel) -> Vec<u32> {
    span_depths_for(model.spans())
}

fn span_depths_for(spans: &[NvtxSpan]) -> Vec<u32> {
    let mut depths: Vec<Option<u32>> = vec![None; spans.len()];
    let mut chain_positions: Vec<Option<usize>> = vec![None; spans.len()];

    for start in 0..spans.len() {
        if depths[start].is_some() {
            continue;
        }

        let mut chain = Vec::new();
        let mut cursor = Some(start);
        let mut next_depth = 0_u32;
        while let Some(index) = cursor {
            if index >= spans.len() {
                break;
            }
            if let Some(depth) = depths[index] {
                next_depth = depth.saturating_add(1);
                break;
            }
            if let Some(cycle_start) = chain_positions[index] {
                for cycle_index in chain.drain(cycle_start..) {
                    depths[cycle_index] = Some(0);
                    chain_positions[cycle_index] = None;
                }
                next_depth = 1;
                break;
            }

            chain_positions[index] = Some(chain.len());
            chain.push(index);
            cursor = spans[index]
                .kind
                .parent()
                .map(|SpanId(parent_index)| parent_index);
        }

        for index in chain.into_iter().rev() {
            depths[index] = Some(next_depth);
            chain_positions[index] = None;
            next_depth = next_depth.saturating_add(1);
        }
    }

    depths.into_iter().map(|depth| depth.unwrap_or(0)).collect()
}

fn range_item(
    model: &NvtxModel,
    domain: &NvtxCatalogDomain,
    span: &NvtxSpan,
    query_start: TimeUnixNanoSec,
    viewport: AbsoluteViewport,
) -> Option<NvtxRangeItem> {
    let effective_end = span.end.unwrap_or(model.trace_end());
    let thread_id = span.kind.thread_id();
    let kind = match span.kind {
        SpanKind::PushPop { .. } => NvtxRangeKind::PushPop,
        SpanKind::StartEnd => NvtxRangeKind::StartEnd,
        SpanKind::Resource { .. } => return None,
    };
    Some(NvtxRangeItem {
        message: span.name.clone(),
        domain_id: span.domain,
        domain_name: domain.name.clone(),
        category_id: span.category,
        category_name: span
            .category
            .and_then(|id| model.category_name(span.domain, id)),
        color: display_color(span.color, span.domain),
        kind,
        thread_id,
        thread_name: thread_id.map(|id| model.thread_name(id)),
        observed_start: to_secs_relative(span.start, query_start),
        observed_end: span.end.map(|end| to_secs_relative(end, query_start)),
        display_start: to_secs_relative(span.start.max(viewport.start), query_start),
        display_end: to_secs_relative(effective_end.min(viewport.end), query_start),
        observed_duration: span.duration().map(to_secs),
        incomplete: span.end.is_none(),
    })
}

fn sort_ranges(ranges: &mut [NvtxRangeItem]) {
    ranges.sort_by(|left, right| {
        left.display_start
            .total_cmp(&right.display_start)
            .then(left.display_end.total_cmp(&right.display_end))
            .then(left.message.cmp(&right.message))
    });
}

fn display_color(color: Option<NvtxColor>, domain_id: u64) -> String {
    match color {
        Some(NvtxColor {
            color_type: 1,
            value,
        }) => {
            let alpha = value >> 24;
            let red = (value >> 16) & 0xff;
            let green = (value >> 8) & 0xff;
            let blue = value & 0xff;
            format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
        }
        _ => fallback_color(domain_id).to_owned(),
    }
}

fn fallback_color(domain_id: u64) -> &'static str {
    const COLORS: [&str; 12] = [
        "#2563eb", "#7c3aed", "#db2777", "#dc2626", "#ea580c", "#ca8a04", "#16a34a", "#0d9488",
        "#0891b2", "#4f46e5", "#9333ea", "#475569",
    ];
    COLORS[(domain_id % COLORS.len() as u64) as usize]
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StatsGroupKey {
    domain_id: u64,
    category_id: Option<u32>,
    message: String,
}

#[derive(Debug, Default)]
struct StatisticsAccumulator {
    count: u64,
    observed_count: u64,
    total_duration: u64,
    min_duration: Option<u64>,
    max_duration: Option<u64>,
    saturated: bool,
}

impl StatisticsAccumulator {
    fn accumulate(&mut self, span: &NvtxSpan, viewport: AbsoluteViewport) {
        self.count = self.count.saturating_add(1);
        let Some(end) = span.end else {
            return;
        };
        let duration = end
            .min(viewport.end)
            .saturating_sub(span.start.max(viewport.start));
        self.min_duration = Some(
            self.min_duration
                .map_or(duration, |minimum| minimum.min(duration)),
        );
        self.max_duration = Some(
            self.max_duration
                .map_or(duration, |maximum| maximum.max(duration)),
        );
        self.observed_count = self.observed_count.saturating_add(1);
        match self.total_duration.checked_add(duration) {
            Some(total) => self.total_duration = total,
            None => {
                self.total_duration = u64::MAX;
                self.saturated = true;
            }
        }
    }

    fn finish(
        self,
        key: &StatsGroupKey,
        domain: &NvtxCatalogDomain,
        model: &NvtxModel,
    ) -> NvtxRangeStatistics {
        let total_duration = to_secs(self.total_duration);
        NvtxRangeStatistics {
            message: key.message.clone(),
            domain_id: key.domain_id,
            domain_name: domain.name.clone(),
            category_id: key.category_id,
            category_name: key
                .category_id
                .and_then(|id| model.category_name(key.domain_id, id)),
            count: self.count,
            observed_count: self.observed_count,
            total_duration,
            avg_duration: if self.observed_count == 0 {
                0.0
            } else {
                total_duration / self.observed_count as f64
            },
            min_duration: self.min_duration.map(to_secs),
            max_duration: self.max_duration.map(to_secs),
            saturated: self.saturated,
        }
    }
}

mod decimal_u64 {
    use serde::{Deserialize, Deserializer, Serializer, de};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use nvtx_analyzer::NvtxModelBuilder;
    use nvtx_bridge::NvtxEventEntity;
    use nvtx_events::{NvtxColor, NvtxEvent, NvtxEventAttributes, NvtxMessage};
    use quent_events::Event;
    use uuid::Uuid;

    use super::*;

    const QUERY_START_NS: u64 = 1_750_000_000_000_000_000;

    fn event(timestamp: u64, event: NvtxEvent) -> Event<NvtxEventEntity> {
        Event::new(Uuid::nil(), timestamp, NvtxEventEntity(event))
    }

    fn attributes(name: &str, category: u32, color: Option<NvtxColor>) -> NvtxEventAttributes {
        NvtxEventAttributes {
            category,
            color,
            message: Some(NvtxMessage::String(name.to_owned())),
            payload: None,
        }
    }

    fn seconds(nanoseconds: u64) -> f64 {
        to_secs(nanoseconds)
    }

    fn query_event(offset: u64, nvtx_event: NvtxEvent) -> Event<NvtxEventEntity> {
        event(QUERY_START_NS + offset, nvtx_event)
    }

    fn query_event_signed(offset: i64, nvtx_event: NvtxEvent) -> Event<NvtxEventEntity> {
        let magnitude = offset.unsigned_abs();
        let timestamp = if offset.is_negative() {
            QUERY_START_NS - magnitude
        } else {
            QUERY_START_NS + magnitude
        };
        event(timestamp, nvtx_event)
    }

    fn model() -> NvtxModel {
        NvtxModelBuilder::build(vec![
            query_event(
                100,
                NvtxEvent::RangePush {
                    domain: 2,
                    thread_id: 7,
                    attributes: attributes("outer", 3, None),
                },
            ),
            query_event(
                120,
                NvtxEvent::RangePush {
                    domain: 2,
                    thread_id: 7,
                    attributes: attributes(
                        "inner",
                        3,
                        Some(NvtxColor {
                            color_type: 1,
                            value: 0x8040_2010,
                        }),
                    ),
                },
            ),
            query_event(
                180,
                NvtxEvent::RangePop {
                    domain: 2,
                    thread_id: 7,
                },
            ),
            query_event(
                200,
                NvtxEvent::RangePop {
                    domain: 2,
                    thread_id: 7,
                },
            ),
            query_event(
                210,
                NvtxEvent::RangeStart {
                    domain: 2,
                    range_id: 9,
                    attributes: attributes("open", 0, None),
                },
            ),
            query_event(
                250,
                NvtxEvent::Mark {
                    domain: 2,
                    attributes: attributes("boundary", 0, None),
                },
            ),
            query_event(
                250,
                NvtxEvent::RangeStart {
                    domain: 2,
                    range_id: 10,
                    attributes: attributes("instant", 0, None),
                },
            ),
            query_event(
                250,
                NvtxEvent::RangeEnd {
                    domain: 2,
                    range_id: 10,
                },
            ),
        ])
    }

    #[test]
    fn canonical_selection_rules_are_enforced() {
        let catalog = NvtxCatalog::from_model(&model(), QUERY_START_NS);
        let canonical = catalog
            .canonicalize_request(NvtxViewportRequest {
                viewport: NvtxViewportWindow {
                    start: 0.0,
                    end: seconds(250),
                },
                selections: vec![NvtxDomainSelection {
                    domain_id: 2,
                    category_ids: vec![3, 3],
                    include_uncategorized: true,
                }],
            })
            .expect("valid selection");
        assert_eq!(canonical.selections[0].category_ids, vec![3]);

        let duplicate = catalog.canonicalize_request(NvtxViewportRequest {
            viewport: canonical.viewport,
            selections: vec![
                canonical.selections[0].clone(),
                canonical.selections[0].clone(),
            ],
        });
        assert!(matches!(
            duplicate,
            Err(NvtxViewportError::DuplicateDomain { .. })
        ));

        let empty = catalog.canonicalize_request(NvtxViewportRequest {
            viewport: canonical.viewport,
            selections: vec![NvtxDomainSelection {
                domain_id: 2,
                category_ids: vec![],
                include_uncategorized: false,
            }],
        });
        assert!(matches!(
            empty,
            Err(NvtxViewportError::EmptySelection { .. })
        ));

        let inverted = catalog.canonicalize_request(NvtxViewportRequest {
            viewport: NvtxViewportWindow {
                start: seconds(150),
                end: 0.0,
            },
            selections: vec![],
        });
        assert!(matches!(inverted, Err(NvtxViewportError::InvalidWindow)));

        for viewport in [
            NvtxViewportWindow {
                start: f64::NAN,
                end: 1.0,
            },
            NvtxViewportWindow {
                start: 0.0,
                end: f64::INFINITY,
            },
        ] {
            assert!(matches!(
                catalog.canonicalize_request(NvtxViewportRequest {
                    viewport,
                    selections: vec![],
                }),
                Err(NvtxViewportError::InvalidWindow)
            ));
        }

        let unknown_domain = catalog.canonicalize_request(NvtxViewportRequest {
            viewport: canonical.viewport,
            selections: vec![NvtxDomainSelection {
                domain_id: 4_242,
                category_ids: vec![3],
                include_uncategorized: false,
            }],
        });
        assert!(matches!(
            unknown_domain,
            Err(NvtxViewportError::UnknownDomain { domain_id: 4_242 })
        ));

        let unknown_category = catalog.canonicalize_request(NvtxViewportRequest {
            viewport: canonical.viewport,
            selections: vec![NvtxDomainSelection {
                domain_id: 2,
                category_ids: vec![4_242],
                include_uncategorized: false,
            }],
        });
        assert!(matches!(
            unknown_category,
            Err(NvtxViewportError::UnknownCategory {
                domain_id: 2,
                category_id: 4_242
            })
        ));

        let categorized_model = NvtxModelBuilder::build(vec![
            event(
                100,
                NvtxEvent::RangePush {
                    domain: 2,
                    thread_id: 7,
                    attributes: attributes("categorized", 3, None),
                },
            ),
            event(
                200,
                NvtxEvent::RangePop {
                    domain: 2,
                    thread_id: 7,
                },
            ),
        ]);
        let categorized_catalog =
            NvtxCatalog::from_model(&categorized_model, categorized_model.trace_start());
        let uncategorized = categorized_catalog.canonicalize_request(NvtxViewportRequest {
            viewport: canonical.viewport,
            selections: vec![NvtxDomainSelection {
                domain_id: 2,
                category_ids: vec![3],
                include_uncategorized: true,
            }],
        });
        assert!(matches!(
            uncategorized,
            Err(NvtxViewportError::UncategorizedUnavailable { domain_id: 2 })
        ));

        let none = catalog
            .canonicalize_request(NvtxViewportRequest {
                viewport: canonical.viewport,
                selections: vec![],
            })
            .expect("selecting nothing is valid");
        assert!(none.selections.is_empty());
    }

    #[test]
    fn catalog_preserves_reconstruction_anomalies() {
        let model = NvtxModelBuilder::build(vec![
            event(
                100,
                NvtxEvent::RangeEnd {
                    domain: 2,
                    range_id: 99,
                },
            ),
            event(
                110,
                NvtxEvent::RangeStart {
                    domain: 2,
                    range_id: 9,
                    attributes: attributes("displaced", 0, None),
                },
            ),
            event(
                120,
                NvtxEvent::RangeStart {
                    domain: 2,
                    range_id: 9,
                    attributes: attributes("replacement", 0, None),
                },
            ),
        ]);

        let anomalies = NvtxCatalog::from_model(&model, model.trace_start()).anomalies;
        assert_eq!(anomalies.orphan_range_ends, 1);
        assert_eq!(anomalies.reused_range_ids, 1);
        assert_eq!(anomalies.total, 2);
        assert!(!anomalies.is_faithful);
    }

    #[test]
    fn anomaly_counters_are_lossless_json_strings() {
        let anomalies = NvtxCatalogAnomalies {
            orphan_range_ends: u64::MAX,
            orphan_range_pops: u64::MAX,
            orphan_resource_destroys: u64::MAX,
            reused_range_ids: u64::MAX,
            reused_resource_handles: u64::MAX,
            total: u64::MAX,
            is_faithful: false,
        };

        let json = serde_json::to_value(anomalies).expect("anomalies serialize");
        for field in [
            "orphan_range_ends",
            "orphan_range_pops",
            "orphan_resource_destroys",
            "reused_range_ids",
            "reused_resource_handles",
            "total",
        ] {
            assert_eq!(json[field], "18446744073709551615");
        }
        assert_eq!(
            serde_json::from_value::<NvtxCatalogAnomalies>(json).expect("anomalies deserialize"),
            anomalies
        );
    }

    #[test]
    fn request_timing_values_are_decimal_seconds() {
        let request = NvtxViewportRequest {
            viewport: NvtxViewportWindow {
                start: -0.25,
                end: 1.5,
            },
            selections: vec![NvtxDomainSelection {
                domain_id: u64::MAX,
                category_ids: vec![3],
                include_uncategorized: false,
            }],
        };

        let json = serde_json::to_value(&request).expect("request serializes");
        assert_eq!(json["viewport"]["start"], -0.25);
        assert_eq!(json["viewport"]["end"], 1.5);
        assert_eq!(json["selections"][0]["domain_id"], "18446744073709551615");
        assert_eq!(
            serde_json::from_value::<NvtxViewportRequest>(json).expect("request deserializes"),
            request
        );
    }

    #[test]
    fn viewport_preserves_truth_and_clips_display_and_statistics() {
        let model = model();
        let catalog = NvtxCatalog::from_model(&model, QUERY_START_NS);
        assert_eq!(catalog.trace_start, seconds(100));
        assert_eq!(catalog.trace_end, seconds(250));
        assert!(
            serde_json::to_value(&catalog)
                .expect("catalog serializes")
                .get("query_start")
                .is_none()
        );
        let response = NvtxViewportResponse::from_model_with_catalog(
            &model,
            &catalog,
            NvtxViewportRequest {
                viewport: NvtxViewportWindow {
                    start: seconds(110),
                    end: seconds(250),
                },
                selections: catalog.select_all(),
            },
        )
        .expect("valid viewport");
        assert_eq!(response.viewport.start, seconds(110));
        assert_eq!(response.viewport.end, seconds(250));

        let ranges = response.domains[0]
            .lanes
            .iter()
            .flat_map(|lane| &lane.ranges)
            .collect::<Vec<_>>();
        let outer = ranges
            .iter()
            .find(|range| range.message == "outer")
            .unwrap();
        assert_eq!(outer.observed_start, seconds(100));
        assert_eq!(outer.observed_end, Some(seconds(200)));
        assert_eq!(outer.display_start, seconds(110));
        assert_eq!(outer.display_end, seconds(200));
        let outer_json = serde_json::to_value(outer).expect("range serializes");
        assert_eq!(outer_json["observed_start"], seconds(100));
        assert_eq!(outer_json["observed_end"], seconds(200));
        assert_eq!(outer_json["display_start"], seconds(110));
        assert_eq!(outer_json["display_end"], seconds(200));

        let open = ranges.iter().find(|range| range.message == "open").unwrap();
        assert!(open.incomplete);
        assert_eq!(open.observed_end, None);
        assert_eq!(open.display_end, seconds(250));
        assert_eq!(open.observed_duration, None);

        let outer_stats = response
            .statistics
            .iter()
            .find(|stats| stats.message == "outer")
            .unwrap();
        assert_eq!(
            outer_stats.total_duration,
            seconds(90),
            "duration is viewport-clipped"
        );
        let open_stats = response
            .statistics
            .iter()
            .find(|stats| stats.message == "open")
            .unwrap();
        assert_eq!(open_stats.count, 1);
        assert_eq!(open_stats.observed_count, 0);
        assert_eq!(open_stats.total_duration, 0.0);
        assert_eq!(open_stats.min_duration, None);
        assert_eq!(open_stats.max_duration, None);

        let instant = ranges
            .iter()
            .find(|range| range.message == "instant")
            .unwrap();
        assert_eq!(instant.display_start, seconds(250));
        assert_eq!(instant.display_end, seconds(250));
        let instant_stats = response
            .statistics
            .iter()
            .find(|stats| stats.message == "instant")
            .unwrap();
        assert_eq!(instant_stats.observed_count, 1);
        assert_eq!(instant_stats.total_duration, 0.0);
        assert_eq!(instant_stats.min_duration, Some(0.0));
        assert_eq!(instant_stats.max_duration, Some(0.0));
    }

    #[test]
    fn viewport_supports_trace_data_before_query_start() {
        let model = NvtxModelBuilder::build(vec![
            query_event_signed(
                -100,
                NvtxEvent::RangeStart {
                    domain: 2,
                    range_id: 9,
                    attributes: attributes("crosses query start", 0, None),
                },
            ),
            query_event_signed(
                100,
                NvtxEvent::RangeEnd {
                    domain: 2,
                    range_id: 9,
                },
            ),
        ]);
        let catalog = NvtxCatalog::from_model(&model, QUERY_START_NS);
        assert_eq!(catalog.trace_start, -seconds(100));
        assert_eq!(catalog.trace_end, seconds(100));

        let response = NvtxViewportResponse::from_model_with_catalog(
            &model,
            &catalog,
            NvtxViewportRequest {
                viewport: NvtxViewportWindow {
                    start: -seconds(75),
                    end: -seconds(25),
                },
                selections: catalog.select_all(),
            },
        )
        .expect("negative relative viewport is valid");

        let range = &response.domains[0].lanes[0].ranges[0];
        assert_eq!(range.observed_start, -seconds(100));
        assert_eq!(range.observed_end, Some(seconds(100)));
        assert_eq!(range.display_start, -seconds(75));
        assert_eq!(range.display_end, -seconds(25));
        assert_eq!(response.statistics[0].total_duration, seconds(50));
    }

    #[test]
    fn depth_boundary_marks_and_argb_color_are_ui_ready() {
        let model = model();
        let response = NvtxViewportResponse::from_model(
            &model,
            QUERY_START_NS,
            NvtxViewportRequest {
                viewport: NvtxViewportWindow {
                    start: seconds(120),
                    end: seconds(250),
                },
                selections: NvtxCatalog::from_model(&model, QUERY_START_NS).select_all(),
            },
        )
        .expect("valid viewport");
        let lanes = &response.domains[0].lanes;
        assert!(lanes.iter().any(|lane| {
            lane.identity
                == NvtxLaneIdentity::Thread {
                    thread_id: 7,
                    depth: 1,
                }
                && lane.ranges[0].color == "#40201080"
        }));
        assert!(lanes.iter().any(|lane| {
            lane.identity == NvtxLaneIdentity::Marks && lane.marks[0].timestamp == seconds(250)
        }));
    }

    #[test]
    fn nested_span_depths_reuse_resolved_parent_chains() {
        let model = NvtxModelBuilder::build(vec![
            event(
                100,
                NvtxEvent::RangePush {
                    domain: 2,
                    thread_id: 7,
                    attributes: attributes("outer", 0, None),
                },
            ),
            event(
                110,
                NvtxEvent::RangePush {
                    domain: 2,
                    thread_id: 7,
                    attributes: attributes("middle", 0, None),
                },
            ),
            event(
                120,
                NvtxEvent::RangePush {
                    domain: 2,
                    thread_id: 7,
                    attributes: attributes("inner", 0, None),
                },
            ),
            event(
                130,
                NvtxEvent::RangePop {
                    domain: 2,
                    thread_id: 7,
                },
            ),
            event(
                140,
                NvtxEvent::RangePop {
                    domain: 2,
                    thread_id: 7,
                },
            ),
            event(
                150,
                NvtxEvent::RangePop {
                    domain: 2,
                    thread_id: 7,
                },
            ),
        ]);

        let depths = span_depths(&model);
        for (name, expected_depth) in [("outer", 0), ("middle", 1), ("inner", 2)] {
            let index = model
                .spans()
                .iter()
                .position(|span| span.name == name)
                .expect("nested span exists");
            assert_eq!(depths[index], expected_depth);
        }
    }

    fn span_with_parent(parent: Option<SpanId>) -> NvtxSpan {
        NvtxSpan {
            domain: 0,
            name: "span".to_owned(),
            category: None,
            color: None,
            payload: None,
            start: 0,
            end: Some(1),
            kind: SpanKind::PushPop {
                thread_id: 1,
                parent,
            },
        }
    }

    #[test]
    fn malformed_parent_chains_have_safe_depths() {
        let missing_parent = vec![span_with_parent(Some(SpanId(99)))];
        assert_eq!(span_depths_for(&missing_parent), vec![0]);

        let cycle = vec![
            span_with_parent(Some(SpanId(1))),
            span_with_parent(Some(SpanId(0))),
        ];
        assert_eq!(span_depths_for(&cycle), vec![0, 0]);
    }

    #[test]
    fn response_domain_ids_are_lossless_json_strings() {
        let domain_id = u64::MAX;
        let values = [
            serde_json::to_value(NvtxCatalogDomain {
                domain_id,
                name: "domain".to_owned(),
                color: "#000000ff".to_owned(),
                threads: vec![],
                categories: vec![],
                has_uncategorized: false,
            })
            .expect("catalog domain serializes"),
            serde_json::to_value(NvtxDomainLaneGroup {
                domain_id,
                name: "domain".to_owned(),
                color: "#000000ff".to_owned(),
                lanes: vec![],
            })
            .expect("lane group serializes"),
            serde_json::to_value(NvtxRangeItem {
                message: "range".to_owned(),
                domain_id,
                domain_name: "domain".to_owned(),
                category_id: None,
                category_name: None,
                color: "#000000ff".to_owned(),
                kind: NvtxRangeKind::StartEnd,
                thread_id: None,
                thread_name: None,
                observed_start: 0.0,
                observed_end: Some(1.0),
                display_start: 0.0,
                display_end: 1.0,
                observed_duration: Some(1.0),
                incomplete: false,
            })
            .expect("range serializes"),
            serde_json::to_value(NvtxMarkItem {
                message: "mark".to_owned(),
                domain_id,
                domain_name: "domain".to_owned(),
                category_id: None,
                category_name: None,
                color: "#000000ff".to_owned(),
                timestamp: 0.0,
            })
            .expect("mark serializes"),
            serde_json::to_value(NvtxRangeStatistics {
                message: "range".to_owned(),
                domain_id,
                domain_name: "domain".to_owned(),
                category_id: None,
                category_name: None,
                count: 1,
                observed_count: 1,
                total_duration: 1.0,
                avg_duration: 1.0,
                min_duration: Some(1.0),
                max_duration: Some(1.0),
                saturated: false,
            })
            .expect("statistics serialize"),
        ];

        for value in values {
            assert_eq!(value["domain_id"], "18446744073709551615");
        }
    }

    #[test]
    fn generated_contract_uses_numbers_for_relative_seconds() {
        let config = ts_rs::Config::default();
        let declaration = NvtxViewportRequest::decl(&config);
        assert!(declaration.contains("viewport: NvtxViewportWindow"));
        assert!(NvtxCatalog::decl(&config).contains("trace_start: number"));
        assert!(NvtxCatalog::decl(&config).contains("trace_end: number"));
        assert!(NvtxViewportWindow::decl(&config).contains("start: number"));
        assert!(NvtxViewportWindow::decl(&config).contains("end: number"));
        assert!(NvtxDomainSelection::decl(&config).contains("domain_id: string"));
        for declaration in [
            NvtxCatalogDomain::decl(&config),
            NvtxDomainLaneGroup::decl(&config),
            NvtxRangeItem::decl(&config),
            NvtxMarkItem::decl(&config),
            NvtxRangeStatistics::decl(&config),
        ] {
            assert!(declaration.contains("domain_id: string"));
        }
        let anomalies = NvtxCatalogAnomalies::decl(&config);
        for field in [
            "orphan_range_ends",
            "orphan_range_pops",
            "orphan_resource_destroys",
            "reused_range_ids",
            "reused_resource_handles",
            "total",
        ] {
            assert!(anomalies.contains(&format!("{field}: string")));
        }
        assert!(NvtxRangeItem::decl(&config).contains("observed_start: number"));
        assert!(NvtxRangeItem::decl(&config).contains("observed_end: number | null"));
        assert!(NvtxRangeItem::decl(&config).contains("display_start: number"));
        assert!(NvtxRangeItem::decl(&config).contains("display_end: number"));
        assert!(NvtxMarkItem::decl(&config).contains("timestamp: number"));
        assert!(NvtxRangeStatistics::decl(&config).contains("total_duration: number"));
        assert!(NvtxRangeStatistics::decl(&config).contains("avg_duration: number"));
        assert!(NvtxRangeStatistics::decl(&config).contains("min_duration: number | null"));
        assert!(NvtxRangeStatistics::decl(&config).contains("max_duration: number | null"));
    }
}
