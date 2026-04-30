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
    request::{BulkTimelineRequest, SingleTimelineRequest, TimelineConfig, TimelineRequest},
    response::{
        BulkTimelinesResponse, BulkTimelinesResponseEntry, ResourceTimeline,
        ResourceTimelineBinned, ResourceTimelineBinnedByState, SingleTimelineResponse,
    },
};
use tracing::{debug, trace};
use uuid::Uuid;

use crate::error::ServerResult;

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
    any_cache_hit: bool,
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

        if !cache_result.any_cache_hit {
            return self
                .cold_fetch_and_cache(Arc::clone(&analyzer), request, &ctx)
                .await;
        }

        let CacheCheckResult {
            mut entry_chunks,
            chunk_misses,
            ..
        } = cache_result;

        if !chunk_misses.is_empty() {
            self.partial_fetch_and_cache(
                Arc::clone(&analyzer),
                &request.entries,
                &request.app_params,
                &chunk_misses,
                &ctx,
                &mut entry_chunks,
            )
            .await?;
        }

        assemble_bulk_response(entry_chunks, &request.entries, &geometry)
    }

    /// Check the cache for each (entry, chunk) pair in the current viewport.
    async fn check_cache(&self, ctx: &CacheRequestContext<'_>) -> CacheCheckResult {
        let mut entry_chunks: HashMap<String, Vec<SingleTimelineResponse>> = HashMap::new();
        let mut chunk_misses: HashMap<u64, Vec<String>> = HashMap::new();
        let mut any_cache_hit = false;
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
                    any_cache_hit = true;
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
            any_cache_hit,
            hit_count,
            miss_count,
        }
    }

    /// Cold-cache path: fetch all entries in one bulk call, then split and cache each chunk.
    async fn cold_fetch_and_cache<A>(
        &self,
        analyzer: Arc<A>,
        request: BulkTimelineRequest<
            <A as UiAnalyzer>::TimelineGlobalParams,
            <A as UiAnalyzer>::TimelineParams,
        >,
        ctx: &CacheRequestContext<'_>,
    ) -> ServerResult<BulkTimelinesResponse>
    where
        A: UiAnalyzer + Send + Sync + 'static,
        <A as UiAnalyzer>::TimelineGlobalParams: Send + 'static,
        <A as UiAnalyzer>::TimelineParams: Send + 'static,
    {
        debug!(
            n_entries = request.entries.len(),
            zoom_level = ctx.geometry.zoom_level,
            "bulk timeline cold cache: full fetch"
        );

        let response =
            tokio::task::spawn_blocking(move || analyzer.bulk_resource_timeline(request)).await??;

        for (key, entry_resp) in &response.entries {
            if let BulkTimelinesResponseEntry::Ok { config, data, .. } = entry_resp {
                let chunk_resp = SingleTimelineResponse {
                    config: *config,
                    data: data.clone(),
                };
                let chunks = split_response_into_chunks(
                    &chunk_resp,
                    ctx.geometry.first_chunk,
                    ctx.geometry.last_chunk,
                    ctx.geometry.chunk_duration,
                    ctx.geometry.zoom_level,
                    ctx.geometry.epoch,
                    ctx.geometry.engine_end,
                )?;

                for (chunk_idx, chunk) in chunks {
                    let cache_key = ChunkCacheKey {
                        engine_id: ctx.engine_id,
                        params_hash: ctx.entry_hashes[key],
                        zoom_level: ctx.geometry.zoom_level,
                        chunk_idx,
                        num_bins: ctx.geometry.num_bins,
                    };
                    self.chunks.insert(cache_key, chunk).await;
                }
            }
        }

        Ok(response)
    }

    /// Partial-hit path: fetch only the (entry, chunk) pairs that missed the cache.
    async fn partial_fetch_and_cache<A>(
        &self,
        analyzer: Arc<A>,
        entries: &HashMap<String, TimelineRequest<<A as UiAnalyzer>::TimelineParams>>,
        app_params: &<A as UiAnalyzer>::TimelineGlobalParams,
        chunk_misses: &HashMap<u64, Vec<String>>,
        ctx: &CacheRequestContext<'_>,
        entry_chunks: &mut HashMap<String, Vec<SingleTimelineResponse>>,
    ) -> ServerResult<()>
    where
        A: UiAnalyzer + Send + Sync + 'static,
        <A as UiAnalyzer>::TimelineGlobalParams: Clone + Send + 'static,
        <A as UiAnalyzer>::TimelineParams: Clone + Send + 'static,
    {
        let n_miss_entries: usize = chunk_misses.values().map(|v| v.len()).sum();
        debug!(
            n_miss_chunks = chunk_misses.len(),
            n_miss_entries,
            zoom_level = ctx.geometry.zoom_level,
            "bulk timeline partial cache: fetching missing chunks"
        );

        for (chunk_idx, miss_keys) in chunk_misses {
            let chunk_start = ctx.geometry.epoch + chunk_idx * ctx.geometry.chunk_duration;
            let chunk_end = if *chunk_idx == ctx.geometry.zoom_level - 1 {
                ctx.geometry.engine_end
            } else {
                ctx.geometry.epoch + (chunk_idx + 1) * ctx.geometry.chunk_duration
            };
            let timeline_config = TimelineConfig {
                num_bins: ctx.geometry.num_bins,
                start: to_secs_relative(chunk_start, ctx.geometry.epoch),
                end: to_secs_relative(chunk_end, ctx.geometry.epoch),
            };

            let chunk_entries: HashMap<String, TimelineRequest<<A as UiAnalyzer>::TimelineParams>> =
                miss_keys
                    .iter()
                    .map(|key| {
                        (
                            key.clone(),
                            entries[key].clone().with_config(timeline_config.clone()),
                        )
                    })
                    .collect();

            let a = Arc::clone(&analyzer);
            let app_params_clone = app_params.clone();
            let response = tokio::task::spawn_blocking(move || {
                a.bulk_resource_timeline(BulkTimelineRequest {
                    entries: chunk_entries,
                    app_params: app_params_clone,
                })
            })
            .await??;

            for (key, entry_resp) in response.entries {
                if let BulkTimelinesResponseEntry::Ok { config, data, .. } = entry_resp {
                    let single = SingleTimelineResponse { config, data };
                    let cache_key = ChunkCacheKey {
                        engine_id: ctx.engine_id,
                        params_hash: ctx.entry_hashes[&key],
                        zoom_level: ctx.geometry.zoom_level,
                        chunk_idx: *chunk_idx,
                        num_bins: ctx.geometry.num_bins,
                    };
                    self.chunks.insert(cache_key, single.clone()).await;
                    entry_chunks.entry(key).or_default().push(single);
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

/// Splits a full-viewport response into per-chunk responses for caching.
fn split_response_into_chunks(
    response: &SingleTimelineResponse,
    first_chunk: u64,
    last_chunk: u64,
    chunk_duration: u64,
    zoom_level: u64,
    epoch: TimeNanoSec,
    engine_end: TimeNanoSec,
) -> ServerResult<Vec<(u64, SingleTimelineResponse)>> {
    let mut result = Vec::new();

    for chunk_idx in first_chunk..=last_chunk {
        let chunk_start = epoch + chunk_idx * chunk_duration;
        let chunk_end = if chunk_idx == zoom_level - 1 {
            engine_end
        } else {
            epoch + (chunk_idx + 1) * chunk_duration
        };

        let chunk_span = match SpanNanoSec::try_new(chunk_start, chunk_end) {
            Ok(span) => span,
            Err(_) => continue,
        };

        // Find the bin range within the full response that falls inside this chunk.
        let (start_idx, end_idx) = overlap_indices(response, &chunk_span, epoch);
        if start_idx >= end_idx {
            continue;
        }

        let chunk_num_bins = (end_idx - start_idx) as u64;
        let chunk_config = BinnedSpan::try_new(
            chunk_span,
            std::num::NonZero::try_from(chunk_num_bins).map_err(|e| {
                quent_time::TimeError::InvalidArgument(format!("chunk bins must be > 0: {e}"))
            })?,
        )?
        .try_to_secs_relative(epoch)?;

        // Slice the data arrays at the chunk's bin boundaries.
        let chunk_data = match &response.data {
            ResourceTimeline::Binned(data) => {
                let capacities_values = data
                    .capacities_values
                    .iter()
                    .map(|(cap_name, values)| {
                        (cap_name.clone(), values[start_idx..end_idx].to_vec())
                    })
                    .collect();
                ResourceTimeline::Binned(ResourceTimelineBinned {
                    config: chunk_config,
                    capacities_values,
                    long_fsms: data.long_fsms.clone(),
                })
            }
            ResourceTimeline::BinnedByState(data) => {
                let capacities_states_values = data
                    .capacities_states_values
                    .iter()
                    .map(|(cap_name, states)| {
                        let sliced_states = states
                            .iter()
                            .map(|(state_name, values)| {
                                (state_name.clone(), values[start_idx..end_idx].to_vec())
                            })
                            .collect();
                        (cap_name.clone(), sliced_states)
                    })
                    .collect();
                ResourceTimeline::BinnedByState(ResourceTimelineBinnedByState {
                    config: chunk_config,
                    capacities_states_values,
                    long_fsms: data.long_fsms.clone(),
                })
            }
        };

        // Push this chunk into the result.
        result.push((
            chunk_idx,
            SingleTimelineResponse {
                config: chunk_config,
                data: chunk_data,
            },
        ));
    }

    Ok(result)
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

        for chunk in &sorted {
            let (start_idx, end_idx) = overlap_indices(chunk, &req_span, epoch);
            if start_idx >= end_idx {
                continue;
            }
            total_bins += (end_idx - start_idx) as u64;

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

        let config = BinnedSpan::try_new(
            req_span,
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

        for chunk in &sorted {
            let (start_idx, end_idx) = overlap_indices(chunk, &req_span, epoch);
            if start_idx >= end_idx {
                continue;
            }
            total_bins += (end_idx - start_idx) as u64;

            if let ResourceTimeline::Binned(ref data) = chunk.data {
                for (cap_name, values) in &data.capacities_values {
                    combined
                        .entry(cap_name.clone())
                        .or_default()
                        .extend_from_slice(&values[start_idx..end_idx]);
                }
            }
        }

        let config = BinnedSpan::try_new(
            req_span,
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
