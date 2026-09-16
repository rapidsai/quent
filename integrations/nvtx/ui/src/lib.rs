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

/// Selectable metadata for one logical domain.
#[derive(TS, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvtxCatalogDomain {
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    /// Sorted raw NVTX domain handles represented by this logical domain.
    #[serde(with = "decimal_u64_vec")]
    #[ts(type = "string[]")]
    pub source_domain_ids: Vec<u64>,
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

/// One logical domain's selected categories.
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
    /// Sorted raw NVTX domain handles represented by this logical domain.
    #[serde(with = "decimal_u64_vec")]
    #[ts(type = "string[]")]
    pub source_domain_ids: Vec<u64>,
    pub name: String,
    pub color: String,
    pub lanes: Vec<NvtxLane>,
}

/// A truthful NVTX lane identity. Thread depth rows are explicit rather than
/// reconstructed by TypeScript.
#[derive(TS, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NvtxLaneIdentity {
    Thread {
        #[serde(with = "decimal_u64")]
        #[ts(type = "string")]
        source_domain_id: u64,
        thread_id: u32,
        depth: u32,
    },
    Process {
        #[serde(with = "decimal_u64")]
        #[ts(type = "string")]
        source_domain_id: u64,
    },
    Marks {
        #[serde(with = "decimal_u64")]
        #[ts(type = "string")]
        source_domain_id: u64,
    },
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
    /// Presentation-level logical domain identity.
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    /// Raw NVTX domain handle that emitted this item.
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub source_domain_id: u64,
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
    /// Presentation-level logical domain identity.
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    /// Raw NVTX domain handle that emitted this item.
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub source_domain_id: u64,
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
    /// Presentation-level logical domain identity.
    #[serde(with = "decimal_u64")]
    #[ts(type = "string")]
    pub domain_id: u64,
    /// Sorted raw NVTX domain handles contributing to this aggregate.
    #[serde(with = "decimal_u64_vec")]
    #[ts(type = "string[]")]
    pub source_domain_ids: Vec<u64>,
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

/// Presentation identity for a domain. Only non-default domains backed by a
/// captured `DomainCreate` participate in name grouping. The raw-key branch is
/// what keeps the default domain and every unresolved handle distinct even if
/// their rendered text matches an explicitly named domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CatalogDomainKey {
    Named(String),
    Raw(u64),
}

#[derive(Default)]
struct CatalogDomainBuilder {
    name: String,
    source_domain_ids: BTreeSet<u64>,
    thread_ids: BTreeSet<u32>,
    category_names: BTreeMap<u32, BTreeSet<String>>,
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

        let mut categories_by_domain = HashMap::<u64, Vec<(u32, String)>>::new();
        for category in model.categories() {
            categories_by_domain
                .entry(category.domain)
                .or_default()
                .push((category.category, category.name.clone()));
        }

        let mut grouped = BTreeMap::<CatalogDomainKey, CatalogDomainBuilder>::new();
        for domain in model.domains() {
            let metadata = metadata_by_domain
                .remove(&domain.domain)
                .unwrap_or_default();
            let key = if domain.domain != 0 && domain.created.is_some() {
                CatalogDomainKey::Named(domain.name.clone())
            } else {
                CatalogDomainKey::Raw(domain.domain)
            };
            let group = grouped.entry(key).or_default();
            if group.source_domain_ids.is_empty() {
                group.name.clone_from(&domain.name);
            }
            group.source_domain_ids.insert(domain.domain);
            group.thread_ids.extend(metadata.thread_ids);
            group.has_uncategorized |= metadata.has_uncategorized;
            for (category_id, name) in categories_by_domain
                .remove(&domain.domain)
                .unwrap_or_default()
            {
                group
                    .category_names
                    .entry(category_id)
                    .or_default()
                    .insert(name);
            }
        }

        // A recreated raw handle remains one source member because the analyzer
        // deliberately exposes raw-domain records by handle. Recreating the same
        // logical name with a different raw handle contributes another member,
        // independent of whether those lifetimes overlap.
        let mut domains = grouped
            .into_values()
            .map(|group| {
                let source_domain_ids = group.source_domain_ids.into_iter().collect::<Vec<_>>();
                let domain_id = source_domain_ids[0];
                let mut threads = group
                    .thread_ids
                    .into_iter()
                    .map(|thread_id| NvtxCatalogThread {
                        thread_id,
                        name: thread_names.get(&thread_id).map_or_else(
                            || model.thread_name(thread_id),
                            |name| (*name).to_owned(),
                        ),
                    })
                    .collect::<Vec<_>>();
                threads.sort_by(|left, right| {
                    left.name
                        .cmp(&right.name)
                        .then(left.thread_id.cmp(&right.thread_id))
                });

                let mut categories = group
                    .category_names
                    .into_iter()
                    .map(|(category_id, names)| NvtxCatalogCategory {
                        category_id,
                        name: category_display_name(category_id, &names),
                    })
                    .collect::<Vec<_>>();
                categories.sort_by(|left, right| {
                    left.name
                        .cmp(&right.name)
                        .then(left.category_id.cmp(&right.category_id))
                });

                NvtxCatalogDomain {
                    domain_id,
                    source_domain_ids,
                    name: group.name,
                    color: fallback_color(domain_id).to_owned(),
                    threads,
                    categories,
                    has_uncategorized: group.has_uncategorized,
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

fn category_display_name(category_id: u32, names: &BTreeSet<String>) -> String {
    if let Some(name) = names.iter().next().filter(|_| names.len() == 1) {
        return name.clone();
    }
    format!(
        "<category {category_id} has conflicting names: {}>",
        names.iter().cloned().collect::<Vec<_>>().join(" | ")
    )
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
        let domains_by_source: HashMap<_, _> = catalog
            .domains
            .iter()
            .flat_map(|domain| {
                domain
                    .source_domain_ids
                    .iter()
                    .map(move |source_domain_id| (*source_domain_id, domain))
            })
            .collect();
        let depths = span_depths(model);
        let mut statistics = BTreeMap::<StatsGroupKey, StatisticsAccumulator>::new();
        let mut items_by_domain = HashMap::<u64, DomainViewportItems>::new();

        for (index, span) in model.spans().iter().enumerate() {
            let domain = domains_by_source
                .get(&span.domain)
                .expect("every model source domain belongs to one catalog domain");
            let Some(selection) = selections.get(&domain.domain_id) else {
                continue;
            };
            if !is_range(span)
                || !selected(selection, span.category)
                || !intersects(span.start, span.end.unwrap_or(model.trace_end()), viewport)
            {
                continue;
            }

            let Some(item) = range_item(model, domain, span, catalog.query_start, viewport) else {
                continue;
            };
            statistics
                .entry(StatsGroupKey {
                    domain_id: domain.domain_id,
                    category_id: span.category,
                    category_name: span
                        .category
                        .and_then(|id| model.category_name(span.domain, id)),
                    message: span.name.clone(),
                })
                .or_default()
                .accumulate(span, viewport, span.domain);
            let domain_items = items_by_domain.entry(domain.domain_id).or_default();
            match span.kind {
                SpanKind::PushPop { thread_id, .. } => {
                    domain_items
                        .thread_lanes
                        .entry((span.domain, thread_id, depths[index]))
                        .or_default()
                        .push(item);
                }
                SpanKind::StartEnd => domain_items
                    .process_lanes
                    .entry(span.domain)
                    .or_default()
                    .push(item),
                SpanKind::Resource { .. } => continue,
            }
        }

        for mark in model.marks() {
            let domain = domains_by_source
                .get(&mark.domain)
                .expect("every model source domain belongs to one catalog domain");
            let Some(selection) = selections.get(&domain.domain_id) else {
                continue;
            };
            if !selected(selection, mark.category)
                || mark.timestamp < viewport.start
                || mark.timestamp > viewport.end
            {
                continue;
            }
            items_by_domain
                .entry(domain.domain_id)
                .or_default()
                .mark_lanes
                .entry(mark.domain)
                .or_default()
                .push(NvtxMarkItem {
                    message: mark.name.clone(),
                    domain_id: domain.domain_id,
                    source_domain_id: mark.domain,
                    domain_name: domain.name.clone(),
                    category_id: mark.category,
                    category_name: mark
                        .category
                        .and_then(|id| model.category_name(mark.domain, id)),
                    color: display_color(mark.color, domain.domain_id),
                    timestamp: to_secs_relative(mark.timestamp, catalog.query_start),
                });
        }

        let mut domains = Vec::new();
        for domain in &catalog.domains {
            let Some(domain_items) = items_by_domain.remove(&domain.domain_id) else {
                continue;
            };

            let thread_order: HashMap<_, _> = domain
                .threads
                .iter()
                .enumerate()
                .map(|(index, thread)| (thread.thread_id, index))
                .collect();
            let mut lane_entries: Vec<_> = domain_items.thread_lanes.into_iter().collect();
            lane_entries.sort_by(
                |((left_source, left_thread, left_depth), _),
                 ((right_source, right_thread, right_depth), _)| {
                    thread_order
                        .get(left_thread)
                        .cmp(&thread_order.get(right_thread))
                        .then(left_source.cmp(right_source))
                        .then(left_depth.cmp(right_depth))
                },
            );

            let mut lanes = lane_entries
                .into_iter()
                .map(|((source_domain_id, thread_id, depth), mut ranges)| {
                    sort_ranges(&mut ranges);
                    let thread_name = model.thread_name(thread_id);
                    let source_suffix = source_lane_suffix(domain, source_domain_id);
                    NvtxLane {
                        id: format!(
                            "nvtx:{}:source:{source_domain_id}:thread:{thread_id}:depth:{depth}",
                            domain.domain_id
                        ),
                        label: if depth == 0 {
                            format!("{thread_name}{source_suffix}")
                        } else {
                            format!("{thread_name} · depth {depth}{source_suffix}")
                        },
                        identity: NvtxLaneIdentity::Thread {
                            source_domain_id,
                            thread_id,
                            depth,
                        },
                        ranges,
                        marks: Vec::new(),
                    }
                })
                .collect::<Vec<_>>();

            for (source_domain_id, mut ranges) in domain_items.process_lanes {
                sort_ranges(&mut ranges);
                lanes.push(NvtxLane {
                    id: format!(
                        "nvtx:{}:source:{source_domain_id}:process",
                        domain.domain_id
                    ),
                    label: format!(
                        "Process ranges{}",
                        source_lane_suffix(domain, source_domain_id)
                    ),
                    identity: NvtxLaneIdentity::Process { source_domain_id },
                    ranges,
                    marks: Vec::new(),
                });
            }
            for (source_domain_id, mut marks) in domain_items.mark_lanes {
                marks.sort_by(|left, right| {
                    left.timestamp
                        .total_cmp(&right.timestamp)
                        .then(left.message.cmp(&right.message))
                });
                lanes.push(NvtxLane {
                    id: format!("nvtx:{}:source:{source_domain_id}:marks", domain.domain_id),
                    label: format!("Marks{}", source_lane_suffix(domain, source_domain_id)),
                    identity: NvtxLaneIdentity::Marks { source_domain_id },
                    ranges: Vec::new(),
                    marks,
                });
            }
            if !lanes.is_empty() {
                domains.push(NvtxDomainLaneGroup {
                    domain_id: domain.domain_id,
                    source_domain_ids: domain.source_domain_ids.clone(),
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
                accumulator.finish(&key, domain)
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
    thread_lanes: BTreeMap<(u64, u32, u32), Vec<NvtxRangeItem>>,
    process_lanes: BTreeMap<u64, Vec<NvtxRangeItem>>,
    mark_lanes: BTreeMap<u64, Vec<NvtxMarkItem>>,
}

fn source_lane_suffix(domain: &NvtxCatalogDomain, source_domain_id: u64) -> String {
    if domain.source_domain_ids.len() > 1 {
        format!(" · source {source_domain_id}")
    } else {
        String::new()
    }
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
        domain_id: domain.domain_id,
        source_domain_id: span.domain,
        domain_name: domain.name.clone(),
        category_id: span.category,
        category_name: span
            .category
            .and_then(|id| model.category_name(span.domain, id)),
        color: display_color(span.color, domain.domain_id),
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
    category_name: Option<String>,
    message: String,
}

#[derive(Debug, Default)]
struct StatisticsAccumulator {
    source_domain_ids: BTreeSet<u64>,
    count: u64,
    observed_count: u64,
    total_duration: u64,
    min_duration: Option<u64>,
    max_duration: Option<u64>,
    saturated: bool,
}

impl StatisticsAccumulator {
    fn accumulate(&mut self, span: &NvtxSpan, viewport: AbsoluteViewport, source_domain_id: u64) {
        self.source_domain_ids.insert(source_domain_id);
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

    fn finish(self, key: &StatsGroupKey, domain: &NvtxCatalogDomain) -> NvtxRangeStatistics {
        let total_duration = to_secs(self.total_duration);
        NvtxRangeStatistics {
            message: key.message.clone(),
            domain_id: key.domain_id,
            source_domain_ids: self.source_domain_ids.into_iter().collect(),
            domain_name: domain.name.clone(),
            category_id: key.category_id,
            category_name: key.category_name.clone(),
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

mod decimal_u64_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

    pub fn serialize<S>(values: &[u64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        values
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<String>::deserialize(deserializer)?
            .into_iter()
            .map(|value| value.parse().map_err(de::Error::custom))
            .collect()
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

    fn registered_attributes(handle: u64, category: u32) -> NvtxEventAttributes {
        NvtxEventAttributes {
            category,
            message: Some(NvtxMessage::RegisteredHandle(handle)),
            ..Default::default()
        }
    }

    fn grouped_model() -> NvtxModel {
        const REGISTERED_HANDLE: u64 = 0xCAFE;
        NvtxModelBuilder::build(vec![
            event(
                10,
                NvtxEvent::DomainCreate {
                    domain: 5,
                    name: "CCCL".to_owned(),
                },
            ),
            event(
                11,
                NvtxEvent::DomainCreate {
                    domain: 172,
                    name: "CCCL".to_owned(),
                },
            ),
            event(
                12,
                NvtxEvent::NameThread {
                    thread_id: 42,
                    name: "worker".to_owned(),
                },
            ),
            event(
                13,
                NvtxEvent::NameCategory {
                    domain: 5,
                    category: 7,
                    name: "Compute".to_owned(),
                },
            ),
            event(
                14,
                NvtxEvent::NameCategory {
                    domain: 172,
                    category: 7,
                    name: "Compute".to_owned(),
                },
            ),
            event(
                15,
                NvtxEvent::NameCategory {
                    domain: 5,
                    category: 9,
                    name: "Encode".to_owned(),
                },
            ),
            event(
                16,
                NvtxEvent::NameCategory {
                    domain: 172,
                    category: 9,
                    name: "Decode".to_owned(),
                },
            ),
            event(
                17,
                NvtxEvent::RegisterString {
                    domain: 5,
                    handle: REGISTERED_HANDLE,
                    string: "five outer".to_owned(),
                },
            ),
            event(
                18,
                NvtxEvent::RegisterString {
                    domain: 172,
                    handle: REGISTERED_HANDLE,
                    string: "one-seventy-two outer".to_owned(),
                },
            ),
            event(
                100,
                NvtxEvent::RangePush {
                    domain: 5,
                    thread_id: 42,
                    attributes: registered_attributes(REGISTERED_HANDLE, 7),
                },
            ),
            event(
                110,
                NvtxEvent::RangePush {
                    domain: 172,
                    thread_id: 42,
                    attributes: registered_attributes(REGISTERED_HANDLE, 7),
                },
            ),
            event(
                120,
                NvtxEvent::RangePush {
                    domain: 5,
                    thread_id: 42,
                    attributes: attributes("five inner", 7, None),
                },
            ),
            event(
                130,
                NvtxEvent::RangePush {
                    domain: 172,
                    thread_id: 42,
                    attributes: attributes("one-seventy-two inner", 7, None),
                },
            ),
            event(
                140,
                NvtxEvent::RangePop {
                    domain: 5,
                    thread_id: 42,
                },
            ),
            event(
                150,
                NvtxEvent::RangePop {
                    domain: 172,
                    thread_id: 42,
                },
            ),
            event(
                160,
                NvtxEvent::RangePop {
                    domain: 5,
                    thread_id: 42,
                },
            ),
            event(
                170,
                NvtxEvent::RangePop {
                    domain: 172,
                    thread_id: 42,
                },
            ),
            event(
                180,
                NvtxEvent::RangeStart {
                    domain: 5,
                    range_id: 99,
                    attributes: attributes("shared", 7, None),
                },
            ),
            event(
                181,
                NvtxEvent::RangeStart {
                    domain: 172,
                    range_id: 99,
                    attributes: attributes("shared", 7, None),
                },
            ),
            event(
                200,
                NvtxEvent::Mark {
                    domain: 5,
                    attributes: attributes("five mark", 7, None),
                },
            ),
            event(
                201,
                NvtxEvent::Mark {
                    domain: 172,
                    attributes: attributes("one-seventy-two mark", 7, None),
                },
            ),
            event(
                220,
                NvtxEvent::RangeEnd {
                    domain: 5,
                    range_id: 99,
                },
            ),
            event(
                230,
                NvtxEvent::RangeEnd {
                    domain: 172,
                    range_id: 99,
                },
            ),
            event(
                240,
                NvtxEvent::RangeStart {
                    domain: 5,
                    range_id: 100,
                    attributes: attributes("conflict", 9, None),
                },
            ),
            event(
                241,
                NvtxEvent::RangeStart {
                    domain: 172,
                    range_id: 100,
                    attributes: attributes("conflict", 9, None),
                },
            ),
            event(
                260,
                NvtxEvent::RangeEnd {
                    domain: 5,
                    range_id: 100,
                },
            ),
            event(
                261,
                NvtxEvent::RangeEnd {
                    domain: 172,
                    range_id: 100,
                },
            ),
        ])
    }

    fn whole_trace_request(
        model: &NvtxModel,
        selections: Vec<NvtxDomainSelection>,
    ) -> NvtxViewportRequest {
        NvtxViewportRequest {
            viewport: NvtxViewportWindow {
                start: to_secs_relative(model.trace_start(), 0),
                end: to_secs_relative(model.trace_end(), 0),
            },
            selections,
        }
    }

    #[test]
    fn exact_named_domains_group_after_raw_resolution_and_reconstruction() {
        let model = grouped_model();
        let catalog = NvtxCatalog::from_model(&model, 0);
        assert_eq!(catalog.domains.len(), 1);
        let domain = &catalog.domains[0];
        assert_eq!(domain.domain_id, 5);
        assert_eq!(domain.source_domain_ids, vec![5, 172]);
        assert_eq!(domain.name, "CCCL");
        assert_eq!(
            domain.threads,
            vec![NvtxCatalogThread {
                thread_id: 42,
                name: "worker".to_owned(),
            }]
        );
        assert_eq!(
            domain
                .categories
                .iter()
                .find(|category| category.category_id == 7)
                .unwrap()
                .name,
            "Compute"
        );
        assert_eq!(
            domain
                .categories
                .iter()
                .find(|category| category.category_id == 9)
                .unwrap()
                .name,
            "<category 9 has conflicting names: Decode | Encode>"
        );

        let response = NvtxViewportResponse::from_model_with_catalog(
            &model,
            &catalog,
            whole_trace_request(&model, catalog.select_all()),
        )
        .expect("grouped viewport is valid");
        assert_eq!(response.domains.len(), 1);
        let group = &response.domains[0];
        assert_eq!(group.domain_id, 5);
        assert_eq!(group.source_domain_ids, vec![5, 172]);

        let ranges = group
            .lanes
            .iter()
            .flat_map(|lane| &lane.ranges)
            .collect::<Vec<_>>();
        let marks = group
            .lanes
            .iter()
            .flat_map(|lane| &lane.marks)
            .collect::<Vec<_>>();
        assert_eq!(ranges.len(), 8);
        assert_eq!(marks.len(), 2);
        assert!(ranges.iter().all(|range| range.domain_id == 5));
        assert!(marks.iter().all(|mark| mark.domain_id == 5));
        assert_eq!(
            ranges
                .iter()
                .map(|range| range.source_domain_id)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([5, 172])
        );
        assert!(
            ranges
                .iter()
                .any(|range| { range.source_domain_id == 5 && range.message == "five outer" })
        );
        assert!(ranges.iter().any(|range| {
            range.source_domain_id == 172 && range.message == "one-seventy-two outer"
        }));

        let thread_lanes = group
            .lanes
            .iter()
            .filter_map(|lane| match lane.identity {
                NvtxLaneIdentity::Thread {
                    source_domain_id,
                    thread_id,
                    depth,
                } => Some((source_domain_id, thread_id, depth)),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            thread_lanes,
            BTreeSet::from([(5, 42, 0), (5, 42, 1), (172, 42, 0), (172, 42, 1)])
        );
        assert_eq!(
            group
                .lanes
                .iter()
                .filter_map(|lane| match lane.identity {
                    NvtxLaneIdentity::Process { source_domain_id } => Some(source_domain_id),
                    _ => None,
                })
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([5, 172])
        );
        assert_eq!(
            group
                .lanes
                .iter()
                .filter_map(|lane| match lane.identity {
                    NvtxLaneIdentity::Marks { source_domain_id } => Some(source_domain_id),
                    _ => None,
                })
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([5, 172])
        );
        assert!(
            group
                .lanes
                .iter()
                .flat_map(|lane| lane.ranges.iter())
                .all(|range| range.color == domain.color)
        );

        let shared = response
            .statistics
            .iter()
            .find(|statistics| statistics.message == "shared")
            .expect("matching statistics merge across sources");
        assert_eq!(shared.domain_id, 5);
        assert_eq!(shared.source_domain_ids, vec![5, 172]);
        assert_eq!(shared.count, 2);
        let conflict = response
            .statistics
            .iter()
            .filter(|statistics| statistics.message == "conflict")
            .collect::<Vec<_>>();
        assert_eq!(conflict.len(), 2);
        assert_eq!(
            conflict
                .iter()
                .filter_map(|statistics| statistics.category_name.as_deref())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["Decode", "Encode"])
        );
        assert!(
            conflict
                .iter()
                .all(|statistics| statistics.source_domain_ids.len() == 1)
        );
    }

    #[test]
    fn logical_category_selection_includes_every_raw_member() {
        let model = grouped_model();
        let catalog = NvtxCatalog::from_model(&model, 0);
        let response = NvtxViewportResponse::from_model_with_catalog(
            &model,
            &catalog,
            whole_trace_request(
                &model,
                vec![NvtxDomainSelection {
                    domain_id: 5,
                    category_ids: vec![7],
                    include_uncategorized: false,
                }],
            ),
        )
        .expect("logical category selection is valid");
        let group = &response.domains[0];
        let selected_sources = group
            .lanes
            .iter()
            .flat_map(|lane| {
                lane.ranges
                    .iter()
                    .map(|range| range.source_domain_id)
                    .chain(lane.marks.iter().map(|mark| mark.source_domain_id))
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(selected_sources, BTreeSet::from([5, 172]));
        assert!(
            group
                .lanes
                .iter()
                .flat_map(|lane| lane.ranges.iter())
                .all(|range| range.category_id == Some(7))
        );
        assert!(
            response
                .statistics
                .iter()
                .all(|statistics| statistics.category_id == Some(7))
        );
    }

    #[test]
    fn grouping_keeps_default_unresolved_case_and_recreated_sources_distinct() {
        let model = NvtxModelBuilder::build(vec![
            event(
                10,
                NvtxEvent::DomainCreate {
                    domain: 5,
                    name: "CCCL".to_owned(),
                },
            ),
            event(
                11,
                NvtxEvent::RangeStart {
                    domain: 5,
                    range_id: 1,
                    attributes: attributes("first lifetime", 0, None),
                },
            ),
            event(
                20,
                NvtxEvent::RangeEnd {
                    domain: 5,
                    range_id: 1,
                },
            ),
            event(21, NvtxEvent::DomainDestroy { domain: 5 }),
            event(
                22,
                NvtxEvent::DomainCreate {
                    domain: 172,
                    name: "CCCL".to_owned(),
                },
            ),
            event(
                23,
                NvtxEvent::RangeStart {
                    domain: 172,
                    range_id: 1,
                    attributes: attributes("second lifetime", 0, None),
                },
            ),
            event(
                30,
                NvtxEvent::RangeEnd {
                    domain: 172,
                    range_id: 1,
                },
            ),
            event(
                31,
                NvtxEvent::DomainCreate {
                    domain: 6,
                    name: "cccl".to_owned(),
                },
            ),
            event(
                32,
                NvtxEvent::Mark {
                    domain: 6,
                    attributes: attributes("case-sensitive", 0, None),
                },
            ),
            event(
                33,
                NvtxEvent::Mark {
                    domain: 0,
                    attributes: attributes("default", 0, None),
                },
            ),
            event(
                34,
                NvtxEvent::DomainCreate {
                    domain: 9,
                    name: "default domain".to_owned(),
                },
            ),
            event(
                35,
                NvtxEvent::Mark {
                    domain: 9,
                    attributes: attributes("explicit default text", 0, None),
                },
            ),
            event(
                36,
                NvtxEvent::Mark {
                    domain: 40,
                    attributes: attributes("unresolved forty", 0, None),
                },
            ),
            event(
                37,
                NvtxEvent::Mark {
                    domain: 41,
                    attributes: attributes("unresolved forty-one", 0, None),
                },
            ),
            event(
                40,
                NvtxEvent::DomainCreate {
                    domain: 88,
                    name: "repeat".to_owned(),
                },
            ),
            event(
                41,
                NvtxEvent::RangeStart {
                    domain: 88,
                    range_id: 1,
                    attributes: attributes("before recreation", 0, None),
                },
            ),
            event(
                42,
                NvtxEvent::RangeEnd {
                    domain: 88,
                    range_id: 1,
                },
            ),
            event(43, NvtxEvent::DomainDestroy { domain: 88 }),
            event(
                44,
                NvtxEvent::DomainCreate {
                    domain: 88,
                    name: "repeat".to_owned(),
                },
            ),
            event(
                45,
                NvtxEvent::RangeStart {
                    domain: 88,
                    range_id: 2,
                    attributes: attributes("after recreation", 0, None),
                },
            ),
            event(
                46,
                NvtxEvent::RangeEnd {
                    domain: 88,
                    range_id: 2,
                },
            ),
        ]);
        let catalog = NvtxCatalog::from_model(&model, 0);
        assert_eq!(catalog.domains.len(), 7);

        let by_sources = |sources: &[u64]| {
            catalog
                .domains
                .iter()
                .find(|domain| domain.source_domain_ids == sources)
                .unwrap_or_else(|| panic!("missing catalog domain for sources {sources:?}"))
        };
        assert_eq!(by_sources(&[5, 172]).name, "CCCL");
        assert_eq!(by_sources(&[6]).name, "cccl");
        assert_eq!(by_sources(&[0]).name, "default domain");
        assert_eq!(by_sources(&[9]).name, "default domain");
        assert_eq!(by_sources(&[40]).name, "<domain 0x28>");
        assert_eq!(by_sources(&[41]).name, "<domain 0x29>");
        assert_eq!(by_sources(&[88]).name, "repeat");

        let response = NvtxViewportResponse::from_model_with_catalog(
            &model,
            &catalog,
            whole_trace_request(&model, catalog.select_all()),
        )
        .expect("edge-case viewport is valid");
        let cccl = response
            .domains
            .iter()
            .find(|domain| domain.source_domain_ids == [5, 172])
            .expect("sequential CCCL sources remain present");
        assert_eq!(
            cccl.lanes
                .iter()
                .flat_map(|lane| lane.ranges.iter())
                .map(|range| range.message.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["first lifetime", "second lifetime"])
        );
        let recreated = response
            .domains
            .iter()
            .find(|domain| domain.source_domain_ids == [88])
            .expect("recreated raw handle remains one presentation source");
        assert_eq!(
            recreated
                .lanes
                .iter()
                .flat_map(|lane| lane.ranges.iter())
                .map(|range| range.message.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["after recreation", "before recreation"])
        );
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
                    source_domain_id: 2,
                    thread_id: 7,
                    depth: 1,
                }
                && lane.ranges[0].color == "#40201080"
        }));
        assert!(lanes.iter().any(|lane| {
            lane.identity
                == NvtxLaneIdentity::Marks {
                    source_domain_id: 2,
                }
                && lane.marks[0].timestamp == seconds(250)
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
                source_domain_ids: vec![domain_id],
                name: "domain".to_owned(),
                color: "#000000ff".to_owned(),
                threads: vec![],
                categories: vec![],
                has_uncategorized: false,
            })
            .expect("catalog domain serializes"),
            serde_json::to_value(NvtxDomainLaneGroup {
                domain_id,
                source_domain_ids: vec![domain_id],
                name: "domain".to_owned(),
                color: "#000000ff".to_owned(),
                lanes: vec![],
            })
            .expect("lane group serializes"),
            serde_json::to_value(NvtxRangeItem {
                message: "range".to_owned(),
                domain_id,
                source_domain_id: domain_id,
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
                source_domain_id: domain_id,
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
                source_domain_ids: vec![domain_id],
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
            if value.get("source_domain_id").is_some() {
                assert_eq!(value["source_domain_id"], "18446744073709551615");
            }
            if value.get("source_domain_ids").is_some() {
                assert_eq!(
                    value["source_domain_ids"],
                    serde_json::json!(["18446744073709551615"])
                );
            }
        }

        let lane = serde_json::to_value(NvtxLaneIdentity::Process {
            source_domain_id: domain_id,
        })
        .expect("lane identity serializes");
        assert_eq!(lane["source_domain_id"], "18446744073709551615");
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
        assert!(NvtxCatalogDomain::decl(&config).contains("source_domain_ids: string[]"));
        assert!(NvtxDomainLaneGroup::decl(&config).contains("source_domain_ids: string[]"));
        assert!(NvtxLaneIdentity::decl(&config).contains("source_domain_id: string"));
        assert!(NvtxRangeItem::decl(&config).contains("source_domain_id: string"));
        assert!(NvtxMarkItem::decl(&config).contains("source_domain_id: string"));
        assert!(NvtxRangeStatistics::decl(&config).contains("source_domain_ids: string[]"));
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
