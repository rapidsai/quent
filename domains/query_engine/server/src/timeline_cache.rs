// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};

use moka::future::Cache;
use quent_analyzer::Span;
use quent_query_engine_analyzer::{QueryEngineModel, ui::UiAnalyzer};
use quent_time::{SpanNanoSec, TimeNanoSec, bin::BinnedSpan, to_nanosecs, to_secs_relative};
use quent_ui::timeline::{
    request::{
        BulkChunkedTimelineRequest, BulkTimelineRequest, SingleTimelineRequest, TimelineConfig,
        TimelineRequest,
    },
    response::{
        BulkTimelinesResponse, BulkTimelinesResponseEntry, ResourceTimeline,
        ResourceTimelineBinned, ResourceTimelineBinnedByState, SingleTimelineResponse,
    },
};
use tracing::{debug, trace};
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};

/// Target number of chunks visible in the current view range.
const TARGET_CHUNKS_PER_VIEW: u64 = 2;

/// Newtype wrapper for `f64` that provides `Hash` and `Eq` via bit representation.
/// Two floats are considered equal when their bits are identical (NaN == NaN).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct HashableF64(u64);

impl From<f64> for HashableF64 {
    fn from(v: f64) -> Self {
        Self(v.to_bits())
    }
}

/// View of a timeline entry's identity fields, excluding viewport config.
///
/// The viewport config (`start`, `end`, `num_bins`) is intentionally omitted:
/// `start`/`end` vary with every pan or zoom, and `num_bins` is already a
/// separate field in `ChunkCacheKey`. Only the query-identity fields determine
/// whether two requests map to the same cached chunks.
#[derive(Hash, PartialEq, Eq)]
enum EntryParamsKey<'a, EntryParams> {
    Resource {
        resource_id: Uuid,
        long_entities_threshold_s: Option<HashableF64>,
        entity_type_name: Option<&'a str>,
        application: &'a EntryParams,
    },
    ResourceGroup {
        resource_group_id: Uuid,
        resource_type_name: &'a str,
        long_entities_threshold_s: Option<HashableF64>,
        entity_type_name: Option<&'a str>,
        app_params: &'a EntryParams,
    },
}

impl<'a, EntryParams> EntryParamsKey<'a, EntryParams> {
    fn from_request(entry: &'a TimelineRequest<EntryParams>) -> Self {
        match entry {
            TimelineRequest::Resource(r) => Self::Resource {
                resource_id: r.resource_id,
                long_entities_threshold_s: r.long_entities_threshold_s.map(HashableF64::from),
                entity_type_name: r.entity_filter.entity_type_name.as_deref(),
                application: &r.application,
            },
            TimelineRequest::ResourceGroup(rg) => Self::ResourceGroup {
                resource_group_id: rg.resource_group_id,
                resource_type_name: &rg.resource_type_name,
                long_entities_threshold_s: rg.long_entities_threshold_s.map(HashableF64::from),
                entity_type_name: rg.entity_filter.entity_type_name.as_deref(),
                app_params: &rg.app_params,
            },
        }
    }
}

/// Pairs an entry key with global app params for stable cache key hashing.
#[derive(Hash, PartialEq, Eq)]
struct CacheParamsKey<'a, AppParams, EntryParams> {
    entry: EntryParamsKey<'a, EntryParams>,
    app_params: &'a AppParams,
}

/// Chunk geometry computed from engine metadata and the current viewport.
struct ChunkGeometry {
    epoch: TimeNanoSec,
    engine_end: TimeNanoSec,
    zoom_level: u64,
    chunk_duration: u64,
    first_chunk: u64,
    last_chunk: u64,
    num_bins: u16,
}

/// Result of a bulk cache check: which chunks were hits and which were misses.
struct CacheCheckResult {
    /// Cached chunk responses accumulated per entry key.
    entry_chunks: HashMap<String, Vec<SingleTimelineResponse>>,
    /// Entry keys that missed, grouped by chunk index.
    chunk_misses: HashMap<u64, Vec<String>>,
    hit_count: u64,
    miss_count: u64,
}

/// Identity of a cache lookup: engine, per-entry param hashes, and chunk geometry.
/// Together these uniquely determine the `ChunkCacheKey` for every (entry, chunk) pair.
struct CacheRequestContext<'a> {
    engine_id: Uuid,
    entry_hashes: &'a HashMap<String, u64>,
    geometry: &'a ChunkGeometry,
}

/// Key identifying a cached timeline chunk.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct ChunkCacheKey {
    engine_id: Uuid,
    params_hash: u64,
    zoom_level: u64,
    chunk_idx: u64,
    num_bins: u16,
}

/// Cache for timeline chunk responses.
///
/// Used by both single and bulk timeline endpoints. The same `ChunkCacheKey`
/// structure works for both: the `params_hash` is computed per-entry, so
/// an entry fetched via bulk produces the same cache key as if it were
/// fetched via single (allowing cross-endpoint cache sharing).
#[derive(Clone)]
pub struct TimelineCache {
    chunks: Cache<ChunkCacheKey, SingleTimelineResponse>,
}

impl TimelineCache {
    pub(crate) fn new() -> Self {
        Self {
            chunks: Cache::builder()
                .max_capacity(4096)
                .time_to_live(Duration::from_hours(1))
                .build(),
        }
    }

    /// Fetch bulk timelines, serving as many chunks from cache as possible.
    pub(crate) async fn cached_bulk_timeline<A>(
        &self,
        analyzer: Arc<A>,
        engine_id: Uuid,
        request: BulkTimelineRequest<
            <A as UiAnalyzer>::TimelineGlobalParams,
            <A as UiAnalyzer>::TimelineParams,
        >,
    ) -> ServerResult<BulkTimelinesResponse>
    where
        A: UiAnalyzer + Send + Sync + 'static,
        <A as UiAnalyzer>::TimelineGlobalParams: Hash + Eq + Clone + Send + 'static,
        <A as UiAnalyzer>::TimelineParams: Hash + Eq + Clone + Send + 'static,
    {
        let Some(geometry) = compute_chunk_geometry(&*analyzer, &request)? else {
            return Ok(tokio::task::spawn_blocking(move || {
                analyzer.bulk_resource_timeline(request)
            })
            .await??);
        };

        let entry_hashes = compute_entry_hashes(&request.entries, &request.app_params);
        let ctx = CacheRequestContext {
            engine_id,
            entry_hashes: &entry_hashes,
            geometry: &geometry,
        };
        let cache_result = self.check_cache(&ctx).await;

        debug!(
            hit_count = cache_result.hit_count,
            miss_count = cache_result.miss_count,
            zoom_level = geometry.zoom_level,
            n_entries = request.entries.len(),
            "bulk timeline cache check"
        );

        let CacheCheckResult {
            mut entry_chunks,
            chunk_misses,
            ..
        } = cache_result;

        if !chunk_misses.is_empty() {
            let mut error_entries = HashMap::new();
            self.fetch_missing_chunks(
                Arc::clone(&analyzer),
                &request,
                &chunk_misses,
                &ctx,
                &mut entry_chunks,
                &mut error_entries,
            )
            .await?;

            let mut response = assemble_bulk_response(entry_chunks, &request.entries, &geometry)?;
            response.entries.extend(error_entries);
            return Ok(response);
        }

        assemble_bulk_response(entry_chunks, &request.entries, &geometry)
    }

    /// Check the cache for each (entry, chunk) pair in the current viewport.
    async fn check_cache(&self, ctx: &CacheRequestContext<'_>) -> CacheCheckResult {
        let mut entry_chunks: HashMap<String, Vec<SingleTimelineResponse>> = HashMap::new();
        let mut chunk_misses: HashMap<u64, Vec<String>> = HashMap::new();
        let mut hit_count = 0u64;
        let mut miss_count = 0u64;

        for chunk_idx in ctx.geometry.first_chunk..=ctx.geometry.last_chunk {
            for (key, &params_hash) in ctx.entry_hashes {
                let cache_key = ChunkCacheKey {
                    engine_id: ctx.engine_id,
                    params_hash,
                    zoom_level: ctx.geometry.zoom_level,
                    chunk_idx,
                    num_bins: ctx.geometry.num_bins,
                };

                if let Some(cached) = self.chunks.get(&cache_key).await {
                    hit_count += 1;
                    entry_chunks.entry(key.clone()).or_default().push(cached);
                } else {
                    miss_count += 1;
                    chunk_misses.entry(chunk_idx).or_default().push(key.clone());
                }
            }
        }

        CacheCheckResult {
            entry_chunks,
            chunk_misses,
            hit_count,
            miss_count,
        }
    }

    /// Fetch every (entry, chunk) pair flagged as a miss, in a single chunked
    /// analyzer call. Caches the canonical chunks and accumulates per-entry
    /// chunk responses; per-entry errors land in `error_entries`.
    async fn fetch_missing_chunks<A>(
        &self,
        analyzer: Arc<A>,
        request: &BulkTimelineRequest<
            <A as UiAnalyzer>::TimelineGlobalParams,
            <A as UiAnalyzer>::TimelineParams,
        >,
        chunk_misses: &HashMap<u64, Vec<String>>,
        ctx: &CacheRequestContext<'_>,
        entry_chunks: &mut HashMap<String, Vec<SingleTimelineResponse>>,
        error_entries: &mut HashMap<String, BulkTimelinesResponseEntry>,
    ) -> ServerResult<()>
    where
        A: UiAnalyzer + Send + Sync + 'static,
        <A as UiAnalyzer>::TimelineGlobalParams: Clone + Send + 'static,
        <A as UiAnalyzer>::TimelineParams: Clone + Send + 'static,
    {
        // Union of missed chunk indices, sorted for stable response slot ordering.
        let mut miss_chunk_indices: Vec<u64> = chunk_misses.keys().copied().collect();
        miss_chunk_indices.sort();
        if miss_chunk_indices.is_empty() {
            return Ok(());
        }

        // For each entry that missed at least one chunk, the set of chunk indices
        // it missed. Used to discard responses for pairs the analyzer recomputed
        // redundantly when entries have non-uniform miss patterns (rare).
        let mut entry_miss_chunks: HashMap<String, std::collections::HashSet<u64>> = HashMap::new();
        for (chunk_idx, keys) in chunk_misses {
            for k in keys {
                entry_miss_chunks
                    .entry(k.clone())
                    .or_default()
                    .insert(*chunk_idx);
            }
        }

        let mut miss_entry_keys: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for keys in chunk_misses.values() {
            for k in keys {
                miss_entry_keys.insert(k.clone());
            }
        }

        debug!(
            n_miss_chunks = miss_chunk_indices.len(),
            n_miss_entries = miss_entry_keys.len(),
            zoom_level = ctx.geometry.zoom_level,
            "bulk timeline: fetching missing chunks"
        );

        let configs: Vec<TimelineConfig> = miss_chunk_indices
            .iter()
            .map(|&chunk_idx| {
                let chunk_start = ctx.geometry.epoch + chunk_idx * ctx.geometry.chunk_duration;
                let chunk_end = if chunk_idx == ctx.geometry.zoom_level - 1 {
                    ctx.geometry.engine_end
                } else {
                    ctx.geometry.epoch + (chunk_idx + 1) * ctx.geometry.chunk_duration
                };
                TimelineConfig {
                    num_bins: ctx.geometry.num_bins,
                    start: to_secs_relative(chunk_start, ctx.geometry.epoch),
                    end: to_secs_relative(chunk_end, ctx.geometry.epoch),
                }
            })
            .collect();

        let chunked_entries: HashMap<String, TimelineRequest<<A as UiAnalyzer>::TimelineParams>> =
            miss_entry_keys
                .into_iter()
                .map(|k| {
                    let request = request.entries[&k].clone();
                    (k, request)
                })
                .collect();

        let a = Arc::clone(&analyzer);
        let app_params_clone = request.app_params.clone();
        let response = tokio::task::spawn_blocking(move || {
            a.bulk_chunked_resource_timeline(BulkChunkedTimelineRequest {
                entries: chunked_entries,
                configs,
                app_params: app_params_clone,
            })
        })
        .await??;

        for (key, per_chunk) in response.entries {
            if per_chunk.len() != miss_chunk_indices.len() {
                return Err(ServerError::Cache(format!(
                    "chunked analyzer returned {} slots for entry '{}', expected {}",
                    per_chunk.len(),
                    key,
                    miss_chunk_indices.len()
                )));
            }
            // Skip entries the analyzer recomputed redundantly (mixed miss
            // patterns, rare). We only cache and collect slots that we actually
            // asked for.
            let Some(missed_chunks) = entry_miss_chunks.get(&key) else {
                continue;
            };
            for (slot_idx, entry_resp) in per_chunk.into_iter().enumerate() {
                let chunk_idx = miss_chunk_indices[slot_idx];
                if !missed_chunks.contains(&chunk_idx) {
                    continue;
                }
                match entry_resp {
                    BulkTimelinesResponseEntry::Ok { config, data, .. } => {
                        let single = SingleTimelineResponse { config, data };
                        let cache_key = ChunkCacheKey {
                            engine_id: ctx.engine_id,
                            params_hash: ctx.entry_hashes[&key],
                            zoom_level: ctx.geometry.zoom_level,
                            chunk_idx,
                            num_bins: ctx.geometry.num_bins,
                        };
                        self.chunks.insert(cache_key, single.clone()).await;
                        entry_chunks.entry(key.clone()).or_default().push(single);
                    }
                    BulkTimelinesResponseEntry::Error { message } => {
                        error_entries
                            .insert(key.clone(), BulkTimelinesResponseEntry::Error { message });
                    }
                }
            }
        }

        Ok(())
    }

    pub(crate) async fn cached_single_timeline<A>(
        &self,
        analyzer: Arc<A>,
        engine_id: Uuid,
        request: SingleTimelineRequest<
            <A as UiAnalyzer>::TimelineGlobalParams,
            <A as UiAnalyzer>::TimelineParams,
        >,
    ) -> ServerResult<SingleTimelineResponse>
    where
        A: UiAnalyzer + Send + Sync + 'static,
        <A as UiAnalyzer>::TimelineGlobalParams: Hash + Eq + Clone + Send + 'static,
        <A as UiAnalyzer>::TimelineParams: Hash + Eq + Clone + Send + 'static,
    {
        let engine_span = analyzer.query_engine_model().engine()?.span()?;
        let engine_duration = engine_span.duration();
        let epoch = engine_span.start();

        if engine_duration == 0 {
            return Ok(analyzer.single_resource_timeline(request)?);
        }

        // Convert request seconds to absolute nanoseconds.
        let req_start = epoch + to_nanosecs(request.entry.config().start);
        let req_end = epoch + to_nanosecs(request.entry.config().end);
        let req_span = match SpanNanoSec::try_new(req_start, req_end) {
            Ok(span) => span,
            Err(_) => return Ok(analyzer.single_resource_timeline(request)?),
        };

        // Each chunk uses the same num_bins, so the combined result may contain
        // up to zoom_level * num_bins bins. The response config reflects the
        // actual count, and the frontend adapts accordingly.
        let num_bins = request.entry.config().num_bins;
        let view_duration = req_span.duration();

        if view_duration == 0 {
            return Ok(analyzer.single_resource_timeline(request)?);
        }

        let zoom_level = determine_zoom_level(view_duration, engine_duration);
        let chunk_duration = engine_duration / zoom_level;

        // Hash the entry + app_params for cache key construction.
        // Strip the viewport config before hashing — same reasoning as in cached_bulk_timeline.
        let params_hash = {
            let cache_key = CacheParamsKey {
                entry: EntryParamsKey::from_request(&request.entry),
                app_params: &request.app_params,
            };
            let mut hasher = DefaultHasher::new();
            cache_key.hash(&mut hasher);
            hasher.finish()
        };

        // Compute the range of chunk indices that overlap the request.
        let first_chunk =
            ((req_span.start().saturating_sub(epoch)) / chunk_duration).min(zoom_level - 1);
        let last_chunk = ((req_span.end().saturating_sub(1).saturating_sub(epoch))
            / chunk_duration)
            .min(zoom_level - 1);

        let mut chunk_responses: Vec<SingleTimelineResponse> = Vec::new();
        for chunk_idx in first_chunk..=last_chunk {
            let chunk_start = epoch + chunk_idx * chunk_duration;
            let chunk_end = if chunk_idx == zoom_level - 1 {
                engine_span.end()
            } else {
                epoch + (chunk_idx + 1) * chunk_duration
            };

            let cache_key = ChunkCacheKey {
                engine_id,
                params_hash,
                zoom_level,
                chunk_idx,
                num_bins,
            };

            if let Some(cached) = self.chunks.get(&cache_key).await {
                trace!("timeline chunk cache hit: {cache_key:?}");
                chunk_responses.push(cached);
                continue;
            }

            trace!("timeline chunk cache miss: {cache_key:?}");

            // Convert chunk span back to relative seconds for the request.
            let chunk_request = SingleTimelineRequest {
                entry: request.entry.clone().with_config(TimelineConfig {
                    num_bins,
                    start: to_secs_relative(chunk_start, epoch),
                    end: to_secs_relative(chunk_end, epoch),
                }),
                app_params: request.app_params.clone(),
            };

            let a = Arc::clone(&analyzer);
            let response =
                tokio::task::spawn_blocking(move || a.single_resource_timeline(chunk_request))
                    .await??;
            self.chunks.insert(cache_key, response.clone()).await;
            chunk_responses.push(response);
        }

        if chunk_responses.is_empty() {
            return Ok(analyzer.single_resource_timeline(request)?);
        }

        if chunk_responses.len() == 1 {
            let chunk = chunk_responses.into_iter().next().unwrap();
            let chunk_start_ns = epoch + to_nanosecs(chunk.config.span.start());
            let chunk_end_ns = epoch + to_nanosecs(chunk.config.span.end());
            if chunk_start_ns == req_span.start() && chunk_end_ns == req_span.end() {
                return Ok(chunk);
            }
            return combine_chunks(&[chunk], req_span, epoch);
        }

        combine_chunks(&chunk_responses, req_span, epoch)
    }
}

fn determine_zoom_level(view_duration: TimeNanoSec, total_duration: TimeNanoSec) -> u64 {
    if view_duration == 0 {
        return 1;
    }
    ((total_duration * TARGET_CHUNKS_PER_VIEW) / view_duration).max(1)
}

/// Compute chunk geometry from engine metadata and the current viewport.
///
/// Returns `None` for degenerate requests (empty, zero-duration, invalid span)
/// that should fall through to an uncached bulk fetch.
fn compute_chunk_geometry<A>(
    analyzer: &A,
    request: &BulkTimelineRequest<
        <A as UiAnalyzer>::TimelineGlobalParams,
        <A as UiAnalyzer>::TimelineParams,
    >,
) -> ServerResult<Option<ChunkGeometry>>
where
    A: UiAnalyzer,
{
    let engine_span = analyzer.query_engine_model().engine()?.span()?;
    let engine_duration = engine_span.duration();
    let epoch = engine_span.start();

    if engine_duration == 0 || request.entries.is_empty() {
        return Ok(None);
    }

    // Safety: unwrap OK — empty entries returns None above.
    let timeline_config = request.entries.values().next().unwrap().config();

    let req_start = epoch + to_nanosecs(timeline_config.start);
    let req_end = epoch + to_nanosecs(timeline_config.end);
    let req_span = match SpanNanoSec::try_new(req_start, req_end) {
        Ok(span) => span,
        Err(_) => return Ok(None),
    };

    let view_duration = req_span.duration();
    if view_duration == 0 {
        return Ok(None);
    }

    let zoom_level = determine_zoom_level(view_duration, engine_duration);
    let chunk_duration = engine_duration / zoom_level;

    debug!(
        engine_duration,
        view_duration, zoom_level, "bulk timeline zoom level determined"
    );

    let first_chunk =
        ((req_span.start().saturating_sub(epoch)) / chunk_duration).min(zoom_level - 1);
    let last_chunk = ((req_span.end().saturating_sub(1).saturating_sub(epoch)) / chunk_duration)
        .min(zoom_level - 1);

    Ok(Some(ChunkGeometry {
        epoch,
        engine_end: engine_span.end(),
        zoom_level,
        chunk_duration,
        first_chunk,
        last_chunk,
        num_bins: timeline_config.num_bins,
    }))
}

/// Hash each entry's identity fields (excluding viewport config) into a stable `u64`.
fn compute_entry_hashes<GP, EP>(
    entries: &HashMap<String, TimelineRequest<EP>>,
    app_params: &GP,
) -> HashMap<String, u64>
where
    GP: Hash,
    EP: Hash,
{
    entries
        .iter()
        .map(|(key, entry)| {
            let cache_key = CacheParamsKey {
                entry: EntryParamsKey::from_request(entry),
                app_params,
            };
            let mut hasher = DefaultHasher::new();
            cache_key.hash(&mut hasher);
            (key.clone(), hasher.finish())
        })
        .collect()
}

/// Assemble the final bulk response from the accumulated per-entry chunk slices.
fn assemble_bulk_response<EP>(
    entry_chunks: HashMap<String, Vec<SingleTimelineResponse>>,
    entries: &HashMap<String, TimelineRequest<EP>>,
    geometry: &ChunkGeometry,
) -> ServerResult<BulkTimelinesResponse> {
    let mut result_entries: HashMap<String, BulkTimelinesResponseEntry> = HashMap::new();

    for (key, chunks) in &entry_chunks {
        if chunks.is_empty() {
            continue;
        }

        let config = entries[key].config();
        let chunk_span = match SpanNanoSec::try_new(
            geometry.epoch + to_nanosecs(config.start),
            geometry.epoch + to_nanosecs(config.end),
        ) {
            Ok(span) => span,
            Err(_) => continue,
        };

        let combined = combine_chunks(chunks, chunk_span, geometry.epoch)?;
        result_entries.insert(
            key.clone(),
            BulkTimelinesResponseEntry::Ok {
                message: String::new(),
                config: combined.config,
                data: combined.data,
            },
        );
    }

    Ok(BulkTimelinesResponse {
        entries: result_entries,
    })
}

fn combine_chunks(
    chunks: &[SingleTimelineResponse],
    req_span: SpanNanoSec,
    epoch: TimeNanoSec,
) -> ServerResult<SingleTimelineResponse> {
    let mut sorted: Vec<&SingleTimelineResponse> = chunks.iter().collect();
    sorted.sort_by(|a, b| {
        a.config
            .span
            .start()
            .partial_cmp(&b.config.span.start())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let is_binned_by_state = matches!(&sorted[0].data, ResourceTimeline::BinnedByState(_));

    // Collect long_fsms from all chunks, deduplicated by ID.
    let mut seen_fsm_ids = std::collections::HashSet::new();
    let mut long_fsms = Vec::new();
    for chunk in &sorted {
        let chunk_fsms = match &chunk.data {
            ResourceTimeline::Binned(data) => &data.long_fsms,
            ResourceTimeline::BinnedByState(data) => &data.long_fsms,
        };
        for fsm in chunk_fsms {
            if seen_fsm_ids.insert(fsm.id) {
                long_fsms.push(fsm.clone());
            }
        }
    }

    if is_binned_by_state {
        let mut combined: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<f64>>,
        > = std::collections::HashMap::new();
        let mut total_bins: u64 = 0;
        let mut combined_start: Option<TimeNanoSec> = None;
        let mut combined_end: Option<TimeNanoSec> = None;

        for chunk in &sorted {
            let (start_idx, end_idx) = overlap_indices(chunk, &req_span, epoch);
            if start_idx >= end_idx {
                continue;
            }
            total_bins += (end_idx - start_idx) as u64;
            let (bin_start, bin_end) = selected_bin_span(chunk, start_idx, end_idx, epoch);
            combined_start.get_or_insert(bin_start);
            combined_end = Some(bin_end);

            if let ResourceTimeline::BinnedByState(ref data) = chunk.data {
                for (cap_name, states) in &data.capacities_states_values {
                    let cap_entry = combined.entry(cap_name.clone()).or_default();
                    for (state_name, values) in states {
                        cap_entry
                            .entry(state_name.clone())
                            .or_default()
                            .extend_from_slice(&values[start_idx..end_idx]);
                    }
                }
            }
        }

        let combined_span = SpanNanoSec::try_new(
            combined_start.unwrap_or(req_span.start()),
            combined_end.unwrap_or(req_span.end()),
        )?;
        let config = BinnedSpan::try_new(
            combined_span,
            std::num::NonZero::try_from(total_bins).map_err(|e| {
                quent_time::TimeError::InvalidArgument(format!("combined bins must be > 0: {e}"))
            })?,
        )?
        .try_to_secs_relative(epoch)?;

        Ok(SingleTimelineResponse {
            config,
            data: ResourceTimeline::BinnedByState(ResourceTimelineBinnedByState {
                config,
                capacities_states_values: combined,
                long_fsms,
            }),
        })
    } else {
        let mut combined: std::collections::HashMap<String, Vec<f64>> =
            std::collections::HashMap::new();
        let mut total_bins: u64 = 0;
        let mut combined_start: Option<TimeNanoSec> = None;
        let mut combined_end: Option<TimeNanoSec> = None;

        for chunk in &sorted {
            let (start_idx, end_idx) = overlap_indices(chunk, &req_span, epoch);
            if start_idx >= end_idx {
                continue;
            }
            total_bins += (end_idx - start_idx) as u64;
            let (bin_start, bin_end) = selected_bin_span(chunk, start_idx, end_idx, epoch);
            combined_start.get_or_insert(bin_start);
            combined_end = Some(bin_end);

            if let ResourceTimeline::Binned(ref data) = chunk.data {
                for (cap_name, values) in &data.capacities_values {
                    combined
                        .entry(cap_name.clone())
                        .or_default()
                        .extend_from_slice(&values[start_idx..end_idx]);
                }
            }
        }

        let combined_span = SpanNanoSec::try_new(
            combined_start.unwrap_or(req_span.start()),
            combined_end.unwrap_or(req_span.end()),
        )?;
        let config = BinnedSpan::try_new(
            combined_span,
            std::num::NonZero::try_from(total_bins).map_err(|e| {
                quent_time::TimeError::InvalidArgument(format!("combined bins must be > 0: {e}"))
            })?,
        )?
        .try_to_secs_relative(epoch)?;

        Ok(SingleTimelineResponse {
            config,
            data: ResourceTimeline::Binned(ResourceTimelineBinned {
                config,
                capacities_values: combined,
                long_fsms,
            }),
        })
    }
}

fn selected_bin_span(
    chunk: &SingleTimelineResponse,
    start_idx: usize,
    end_idx: usize,
    epoch: TimeNanoSec,
) -> (TimeNanoSec, TimeNanoSec) {
    let chunk_start = epoch + to_nanosecs(chunk.config.span.start());
    let bin_duration_ns = to_nanosecs(chunk.config.bin_duration);
    (
        chunk_start + (start_idx as u64 * bin_duration_ns),
        chunk_start + (end_idx as u64 * bin_duration_ns),
    )
}

fn overlap_indices(
    chunk: &SingleTimelineResponse,
    req_span: &SpanNanoSec,
    epoch: TimeNanoSec,
) -> (usize, usize) {
    let chunk_start = epoch + to_nanosecs(chunk.config.span.start());
    let chunk_end = epoch + to_nanosecs(chunk.config.span.end());
    let bin_duration_ns = to_nanosecs(chunk.config.bin_duration);
    let num_bins = chunk.config.num_bins as usize;

    let chunk_span = match SpanNanoSec::try_new(chunk_start, chunk_end) {
        Ok(s) => s,
        Err(_) => return (0, 0),
    };

    if !chunk_span.intersects(req_span) || bin_duration_ns == 0 {
        return (0, 0);
    }

    let overlap_start = req_span.start().max(chunk_start);
    let overlap_end = req_span.end().min(chunk_end);

    let start_idx = ((overlap_start - chunk_start) / bin_duration_ns) as usize;
    let end_idx = (overlap_end - chunk_start).div_ceil(bin_duration_ns) as usize;

    (start_idx.min(num_bins), end_idx.min(num_bins))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use quent_analyzer::AnalyzerResult;
    use quent_events::Event;
    use quent_query_engine_analyzer::{
        QueryEngineModel, engine::Engine, model::InMemoryQueryEngineModel, ui::UiAnalyzer,
    };
    use quent_query_engine_model::engine::{EngineEvent, Exit, Init};
    use quent_ui::{
        FiniteStateMachine, FsmTransition,
        timeline::{
            request::{
                BulkTimelineRequest, EntityFilter, ResourceTimelineRequest, TimelineConfig,
                TimelineRequest,
            },
            response::{
                BulkTimelinesResponse, BulkTimelinesResponseEntry, ResourceTimeline,
                ResourceTimelineBinned, SingleTimelineResponse,
            },
        },
    };
    use uuid::Uuid;

    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TestGlobalParams {
        query_id: Uuid,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TestTimelineParams {
        operator_id: Option<Uuid>,
        series_offset: u32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct BulkCallEntry {
        key: String,
        start: f64,
        end: f64,
        operator_id: Option<Uuid>,
    }

    struct TestAnalyzer {
        engine_id: Uuid,
        model: InMemoryQueryEngineModel,
        calls: Mutex<Vec<Vec<BulkCallEntry>>>,
    }

    impl TestAnalyzer {
        fn new() -> Self {
            let engine_id = Uuid::from_u128(1);
            let mut engine = Engine::new(engine_id).unwrap();
            engine.push(Event::new(engine_id, 0, EngineEvent::Init(Init::default())));
            engine.push(Event::new(
                engine_id,
                100_000_000_000,
                EngineEvent::Exit(Exit),
            ));

            Self {
                engine_id,
                model: InMemoryQueryEngineModel {
                    engine,
                    workers: Default::default(),
                    query_groups: Default::default(),
                    queries: Default::default(),
                    plans: Default::default(),
                    operators: Default::default(),
                    ports: Default::default(),
                },
                calls: Mutex::new(Vec::new()),
            }
        }

        fn call_entries(&self) -> Vec<BulkCallEntry> {
            let mut entries = self
                .calls
                .lock()
                .unwrap()
                .iter()
                .flat_map(|call| call.iter().cloned())
                .collect::<Vec<_>>();
            entries.sort_by(|a, b| {
                a.start
                    .partial_cmp(&b.start)
                    .unwrap()
                    .then_with(|| a.key.cmp(&b.key))
            });
            entries
        }

        fn call_keys_by_call(&self) -> Vec<Vec<String>> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .map(|call| {
                    let mut keys = call
                        .iter()
                        .map(|entry| entry.key.clone())
                        .collect::<Vec<_>>();
                    keys.sort();
                    keys
                })
                .collect()
        }
    }

    impl UiAnalyzer for TestAnalyzer {
        type Event = ();
        type EntityRef = ();
        type TimelineGlobalParams = TestGlobalParams;
        type TimelineParams = TestTimelineParams;

        fn try_new(
            _engine_id: Uuid,
            _events: impl Iterator<Item = Event<Self::Event>>,
        ) -> AnalyzerResult<Self>
        where
            Self: Sized,
        {
            unimplemented!("not needed by timeline cache tests")
        }

        fn extract_engine(
            _engine_id: Uuid,
            _events: impl Iterator<Item = Event<Self::Event>>,
        ) -> AnalyzerResult<quent_query_engine_ui::Engine>
        where
            Self: Sized,
        {
            unimplemented!("not needed by timeline cache tests")
        }

        fn query_bundle(
            &self,
            _query_id: Uuid,
        ) -> AnalyzerResult<quent_query_engine_ui::QueryBundle<Self::EntityRef>> {
            unimplemented!("not needed by timeline cache tests")
        }

        fn query_engine_model(&self) -> &impl QueryEngineModel {
            &self.model
        }

        fn single_resource_timeline(
            &self,
            _request: quent_ui::timeline::request::SingleTimelineRequest<
                Self::TimelineGlobalParams,
                Self::TimelineParams,
            >,
        ) -> AnalyzerResult<SingleTimelineResponse> {
            unimplemented!("not needed by bulk cache tests")
        }

        fn bulk_resource_timeline(
            &self,
            request: BulkTimelineRequest<Self::TimelineGlobalParams, Self::TimelineParams>,
        ) -> AnalyzerResult<BulkTimelinesResponse> {
            let mut call = request
                .entries
                .iter()
                .map(|(key, entry)| BulkCallEntry {
                    key: key.clone(),
                    start: entry.config().start,
                    end: entry.config().end,
                    operator_id: entry_params(entry).operator_id,
                })
                .collect::<Vec<_>>();
            call.sort_by(|a, b| a.key.cmp(&b.key));
            self.calls.lock().unwrap().push(call);

            let entries = request
                .entries
                .into_iter()
                .map(|(key, entry)| response_entry(key, entry))
                .collect::<AnalyzerResult<HashMap<_, _>>>()?;

            Ok(BulkTimelinesResponse { entries })
        }
    }

    fn entry_params(entry: &TimelineRequest<TestTimelineParams>) -> &TestTimelineParams {
        match entry {
            TimelineRequest::Resource(req) => &req.application,
            TimelineRequest::ResourceGroup(req) => &req.app_params,
        }
    }

    fn response_entry(
        key: String,
        entry: TimelineRequest<TestTimelineParams>,
    ) -> AnalyzerResult<(String, BulkTimelinesResponseEntry)> {
        let params = entry_params(&entry);
        if params.series_offset == 999 {
            return Ok((
                key,
                BulkTimelinesResponseEntry::Error {
                    message: "bad entry".to_string(),
                },
            ));
        }

        let config = entry.config().try_into_binned_span(0)?;
        let config_secs = config.try_to_secs_relative(0)?;
        let values = (0..config.num_bins.get())
            .map(|idx| {
                let bin = config.bin(idx).unwrap();
                to_secs_relative(bin.start(), 0) + params.series_offset as f64
            })
            .collect::<Vec<_>>();
        let long_fsms = params
            .operator_id
            .map(|id| FiniteStateMachine {
                id,
                type_name: "task".to_string(),
                instance_name: format!("operator-{id}"),
                transitions: vec![
                    FsmTransition {
                        name: "start".to_string(),
                        usages: vec![],
                        timestamp: config_secs.span.start(),
                    },
                    FsmTransition {
                        name: "end".to_string(),
                        usages: vec![],
                        timestamp: config_secs.span.end(),
                    },
                ],
            })
            .into_iter()
            .collect();
        let data = ResourceTimeline::Binned(ResourceTimelineBinned {
            config: config_secs,
            capacities_values: HashMap::from([("capacity".to_string(), values)]),
            long_fsms,
        });

        Ok((
            key,
            BulkTimelinesResponseEntry::Ok {
                message: String::new(),
                config: config_secs,
                data,
            },
        ))
    }

    fn request(
        entries: Vec<(&str, f64, f64, u32, Option<Uuid>)>,
    ) -> BulkTimelineRequest<TestGlobalParams, TestTimelineParams> {
        BulkTimelineRequest {
            entries: entries
                .into_iter()
                .map(|(key, start, end, series_offset, operator_id)| {
                    (
                        key.to_string(),
                        TimelineRequest::Resource(ResourceTimelineRequest {
                            resource_id: Uuid::from_u128(2),
                            long_entities_threshold_s: Some(0.0),
                            entity_filter: EntityFilter {
                                entity_type_name: None,
                            },
                            application: TestTimelineParams {
                                operator_id,
                                series_offset,
                            },
                            config: TimelineConfig {
                                num_bins: 4,
                                start,
                                end,
                            },
                        }),
                    )
                })
                .collect(),
            app_params: TestGlobalParams {
                query_id: Uuid::from_u128(3),
            },
        }
    }

    fn values(response: &BulkTimelinesResponse, key: &str) -> Vec<f64> {
        match response.entries.get(key).unwrap() {
            BulkTimelinesResponseEntry::Ok {
                data: ResourceTimeline::Binned(data),
                ..
            } => data.capacities_values["capacity"].clone(),
            _ => panic!("expected binned ok response"),
        }
    }

    fn response_span(response: &BulkTimelinesResponse, key: &str) -> (f64, f64) {
        match response.entries.get(key).unwrap() {
            BulkTimelinesResponseEntry::Ok { config, .. } => {
                (config.span.start(), config.span.end())
            }
            _ => panic!("expected ok response"),
        }
    }

    fn fsm_ids(response: &BulkTimelinesResponse, key: &str) -> Vec<Uuid> {
        match response.entries.get(key).unwrap() {
            BulkTimelinesResponseEntry::Ok {
                data: ResourceTimeline::Binned(data),
                ..
            } => data.long_fsms.iter().map(|fsm| fsm.id).collect(),
            _ => panic!("expected binned ok response"),
        }
    }

    fn assert_error(response: &BulkTimelinesResponse, key: &str) {
        assert!(
            matches!(
                response.entries.get(key),
                Some(BulkTimelinesResponseEntry::Error { .. })
            ),
            "expected error entry for {key}, got {:?}",
            response.entries
        );
    }

    fn assert_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (idx, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
            assert!(
                (actual - expected).abs() < 1e-9,
                "value at index {idx} differs: actual={actual}, expected={expected}"
            );
        }
    }

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn cold_bulk_fetches_canonical_chunks_not_original_viewport() {
        block_on(async {
            let analyzer = Arc::new(TestAnalyzer::new());
            let cache = TimelineCache::new();

            let response = cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![("a", 30.0, 80.0, 0, None)]),
                )
                .await
                .unwrap();

            let spans = analyzer
                .call_entries()
                .into_iter()
                .map(|call| (call.start, call.end))
                .collect::<Vec<_>>();
            assert_eq!(spans, vec![(25.0, 50.0), (50.0, 75.0), (75.0, 100.0)]);
            assert_close(
                &values(&response, "a"),
                &[25.0, 31.25, 37.5, 43.75, 50.0, 56.25, 62.5, 68.75, 75.0],
            );
            assert_close(
                &[
                    response_span(&response, "a").0,
                    response_span(&response, "a").1,
                ],
                &[25.0, 81.25],
            );
        });
    }

    #[test]
    fn panned_bulk_view_reuses_overlapping_cached_chunks() {
        block_on(async {
            let analyzer = Arc::new(TestAnalyzer::new());
            let cache = TimelineCache::new();

            cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![("a", 25.0, 75.0, 0, None)]),
                )
                .await
                .unwrap();
            let response = cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![("a", 30.0, 80.0, 0, None)]),
                )
                .await
                .unwrap();

            let spans = analyzer
                .call_entries()
                .into_iter()
                .map(|call| (call.start, call.end))
                .collect::<Vec<_>>();
            assert_eq!(spans, vec![(25.0, 50.0), (50.0, 75.0), (75.0, 100.0)]);
            assert_close(
                &values(&response, "a"),
                &[25.0, 31.25, 37.5, 43.75, 50.0, 56.25, 62.5, 68.75, 75.0],
            );
        });
    }

    #[test]
    fn partial_bulk_entry_miss_fetches_only_new_entry_for_cached_chunks() {
        block_on(async {
            let analyzer = Arc::new(TestAnalyzer::new());
            let cache = TimelineCache::new();

            cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![
                        ("a", 25.0, 75.0, 0, None),
                        ("b", 25.0, 75.0, 100, None),
                    ]),
                )
                .await
                .unwrap();
            let response = cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![
                        ("a", 25.0, 75.0, 0, None),
                        ("b", 25.0, 75.0, 100, None),
                        ("c", 25.0, 75.0, 200, None),
                    ]),
                )
                .await
                .unwrap();

            let calls = analyzer.call_keys_by_call();
            assert_eq!(
                calls
                    .iter()
                    .filter(|keys| keys.as_slice() == ["a", "b"])
                    .count(),
                2
            );
            assert_eq!(
                calls.iter().filter(|keys| keys.as_slice() == ["c"]).count(),
                2
            );
            assert_close(
                &values(&response, "c"),
                &[225.0, 231.25, 237.5, 243.75, 250.0, 256.25, 262.5, 268.75],
            );
        });
    }

    #[test]
    fn operator_filter_is_part_of_chunk_cache_key() {
        block_on(async {
            let analyzer = Arc::new(TestAnalyzer::new());
            let cache = TimelineCache::new();
            let first_operator = Uuid::from_u128(6);
            let second_operator = Uuid::from_u128(7);

            cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![("a", 25.0, 75.0, 0, Some(first_operator))]),
                )
                .await
                .unwrap();
            let response = cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![("a", 25.0, 75.0, 0, Some(second_operator))]),
                )
                .await
                .unwrap();

            let operator_ids = analyzer
                .call_entries()
                .into_iter()
                .map(|call| call.operator_id)
                .collect::<Vec<_>>();
            assert_eq!(
                operator_ids
                    .iter()
                    .filter(|id| **id == Some(first_operator))
                    .count(),
                2
            );
            assert_eq!(
                operator_ids
                    .iter()
                    .filter(|id| **id == Some(second_operator))
                    .count(),
                2
            );
            assert_eq!(fsm_ids(&response, "a"), vec![second_operator]);
        });
    }

    #[test]
    fn bulk_entry_errors_are_not_dropped_on_cold_or_partial_miss() {
        block_on(async {
            let analyzer = Arc::new(TestAnalyzer::new());
            let cache = TimelineCache::new();

            let cold_response = cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![("bad", 25.0, 75.0, 999, None)]),
                )
                .await
                .unwrap();
            assert_error(&cold_response, "bad");

            cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![("a", 25.0, 75.0, 0, None)]),
                )
                .await
                .unwrap();
            let partial_response = cache
                .cached_bulk_timeline(
                    Arc::clone(&analyzer),
                    analyzer.engine_id,
                    request(vec![
                        ("a", 25.0, 75.0, 0, None),
                        ("bad", 25.0, 75.0, 999, None),
                    ]),
                )
                .await
                .unwrap();
            assert_error(&partial_response, "bad");
            assert_close(
                &values(&partial_response, "a"),
                &[25.0, 31.25, 37.5, 43.75, 50.0, 56.25, 62.5, 68.75],
            );
        });
    }
}
