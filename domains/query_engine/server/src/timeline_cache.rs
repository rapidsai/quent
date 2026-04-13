// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
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
use serde::Serialize;
use tracing::debug;
use uuid::Uuid;

use crate::error::ServerResult;

/// Target number of chunks visible in the current view range.
const TARGET_CHUNKS_PER_VIEW: u64 = 2;

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

    pub(crate) async fn cached_bulk_timeline<A>(
        &self,
        analyzer: &A,
        engine_id: Uuid,
        request: BulkTimelineRequest<
            <A as UiAnalyzer>::TimelineGlobalParams,
            <A as UiAnalyzer>::TimelineParams,
        >,
    ) -> ServerResult<BulkTimelinesResponse>
    where
        A: UiAnalyzer + Send + Sync + 'static,
        <A as UiAnalyzer>::TimelineGlobalParams: Serialize + Clone,
        <A as UiAnalyzer>::TimelineParams: Serialize + Clone,
    {
        // Fetch engine metadata needed for chunk math.
        let engine_span = analyzer.query_engine_model().engine()?.span()?;
        let engine_duration = engine_span.duration();
        let epoch = engine_span.start();

        // Bypass caching for degenerate cases.
        if engine_duration == 0 || request.entries.is_empty() {
            return Ok(analyzer.bulk_resource_timeline(request)?);
        }

        // All entries share the same viewport config — grab it from the first.
        let timeline_config = request.entries.values().next().unwrap().config();

        let req_start = epoch + to_nanosecs(timeline_config.start);
        let req_end = epoch + to_nanosecs(timeline_config.end);
        let req_span = match SpanNanoSec::try_new(req_start, req_end) {
            Ok(span) => span,
            Err(_) => return Ok(analyzer.bulk_resource_timeline(request)?),
        };

        let num_bins = timeline_config.num_bins;
        let view_duration = req_span.duration();

        if view_duration == 0 {
            return Ok(analyzer.bulk_resource_timeline(request)?);
        }

        // Determine chunk geometry for this zoom level.
        let zoom_level = determine_zoom_level(view_duration, engine_duration);
        let chunk_duration = engine_duration / zoom_level;
        debug!(
            engine_duration,
            view_duration, zoom_level, "bulk timeline zoom level determined"
        );

        // Find which chunk indices overlap the current viewport.
        let first_chunk =
            ((req_span.start().saturating_sub(epoch)) / chunk_duration).min(zoom_level - 1);
        let last_chunk = ((req_span.end().saturating_sub(1).saturating_sub(epoch))
            / chunk_duration)
            .min(zoom_level - 1);

        // Hash each entry's params to build stable cache keys.
        let mut entry_hashes: HashMap<String, u64> = HashMap::new();

        for (key, entry) in &request.entries {
            let params_hash = {
                // Strip the viewport config (start/end/num_bins) before hashing: start/end vary
                // with every pan or zoom, and num_bins is already a separate field in ChunkCacheKey.
                // Only the query-identity fields (resource, filters, app params) should determine
                // whether two requests map to the same cached chunks.
                let canonical = entry.clone().with_config(TimelineConfig {
                    num_bins: 0,
                    start: 0.0,
                    end: 0.0,
                });
                let serialized = serde_json::to_string(&(&canonical, &request.app_params))
                    .map_err(|e| crate::error::ServerError::Cache(e.to_string()))?;
                let mut hasher = DefaultHasher::new();
                serialized.hash(&mut hasher);
                hasher.finish()
            };

            entry_hashes.insert(key.clone(), params_hash);
        }

        // Check the cache for each (entry, chunk) pair.
        // Hits go into entry_chunks; misses are recorded in chunk_misses.
        let mut entry_chunks: HashMap<String, Vec<SingleTimelineResponse>> = HashMap::new();
        let mut chunk_misses: HashMap<u64, Vec<String>> = HashMap::new();
        let mut any_cache_hit = false;
        let mut hit_count = 0u64;
        let mut miss_count = 0u64;

        for chunk_idx in first_chunk..=last_chunk {
            for (key, _entry) in &request.entries {
                let cache_key = ChunkCacheKey {
                    engine_id,
                    params_hash: entry_hashes[key],
                    zoom_level,
                    chunk_idx,
                    num_bins,
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

        debug!(
            hit_count,
            miss_count,
            zoom_level,
            n_entries = request.entries.len(),
            "bulk timeline cache check"
        );

        // Fast path: cold cache — no chunks were cached, so fetch everything in one bulk call.
        if !any_cache_hit {
            debug!(
                n_entries = request.entries.len(),
                zoom_level, "bulk timeline cold cache: full fetch"
            );
            let response = analyzer.bulk_resource_timeline(request)?;
            for (key, entry_resp) in &response.entries {
                if let BulkTimelinesResponseEntry::Ok { config, data, .. } = entry_resp {
                    let chunk_resp = SingleTimelineResponse {
                        config: *config,
                        data: data.clone(),
                    };
                    let chunks = split_response_into_chunks(
                        &chunk_resp,
                        first_chunk,
                        last_chunk,
                        chunk_duration,
                        zoom_level,
                        epoch,
                        engine_span.end(),
                    )?;

                    // Cache each chunk individually.
                    for (chunk_idx, chunk) in chunks {
                        let cache_key = ChunkCacheKey {
                            engine_id,
                            params_hash: entry_hashes[key],
                            zoom_level,
                            chunk_idx,
                            num_bins,
                        };
                        self.chunks.insert(cache_key, chunk).await;
                    }
                }
            }
            return Ok(response);
        }

        let BulkTimelineRequest {
            entries,
            app_params,
        } = request;

        // Partial cache hit: fetch only the missing (entry, chunk) pairs.
        if !chunk_misses.is_empty() {
            let n_miss_entries: usize = chunk_misses.values().map(|v| v.len()).sum();
            debug!(
                n_miss_chunks = chunk_misses.len(),
                n_miss_entries, zoom_level, "bulk timeline partial cache: fetching missing chunks"
            );
        }

        for (chunk_idx, miss_keys) in &chunk_misses {
            let chunk_start = epoch + chunk_idx * chunk_duration;
            let chunk_end = if *chunk_idx == zoom_level - 1 {
                engine_span.end()
            } else {
                epoch + (chunk_idx + 1) * chunk_duration
            };
            let timeline_config = TimelineConfig {
                num_bins,
                start: to_secs_relative(chunk_start, epoch),
                end: to_secs_relative(chunk_end, epoch),
            };

            let mut chunk_entries: HashMap<
                String,
                TimelineRequest<<A as UiAnalyzer>::TimelineParams>,
            > = HashMap::new();

            for key in miss_keys {
                let entry = entries[key].clone().with_config(timeline_config.clone());
                chunk_entries.insert(key.clone(), entry);
            }

            let response = analyzer.bulk_resource_timeline(BulkTimelineRequest {
                entries: chunk_entries,
                app_params: app_params.clone(),
            })?;

            for (key, entry_resp) in response.entries {
                if let BulkTimelinesResponseEntry::Ok { config, data, .. } = entry_resp {
                    let single = SingleTimelineResponse { config, data };
                    let cache_key = ChunkCacheKey {
                        engine_id,
                        params_hash: entry_hashes[&key],
                        zoom_level,
                        chunk_idx: *chunk_idx,
                        num_bins,
                    };
                    self.chunks.insert(cache_key, single.clone()).await;
                    entry_chunks.entry(key).or_default().push(single);
                }
            }
        }

        let mut result_entries: HashMap<String, BulkTimelinesResponseEntry> = HashMap::new();

        // now we have collected all chunks (both cached and fetched) in entry_chunks
        for (key, chunk) in &entry_chunks {
            if chunk.is_empty() {
                continue;
            }

            let config = entries[key].config();
            let chunk_span = match SpanNanoSec::try_new(
                epoch + to_nanosecs(config.start),
                epoch + to_nanosecs(config.end),
            ) {
                Ok(span) => span,
                Err(_) => continue,
            };

            let combined = combine_chunks(chunk, chunk_span, epoch)?;
            let result_entry = BulkTimelinesResponseEntry::Ok {
                message: String::new(),
                config: combined.config,
                data: combined.data,
            };

            result_entries.insert(key.clone(), result_entry);
        }

        Ok(BulkTimelinesResponse {
            entries: result_entries,
        })
    }

    pub(crate) async fn cached_single_timeline<A>(
        &self,
        analyzer: &A,
        engine_id: Uuid,
        request: SingleTimelineRequest<
            <A as UiAnalyzer>::TimelineGlobalParams,
            <A as UiAnalyzer>::TimelineParams,
        >,
    ) -> ServerResult<SingleTimelineResponse>
    where
        A: UiAnalyzer + Send + Sync + 'static,
        <A as UiAnalyzer>::TimelineGlobalParams: Serialize + Clone,
        <A as UiAnalyzer>::TimelineParams: Serialize + Clone,
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
            let canonical = request.entry.clone().with_config(TimelineConfig {
                num_bins: 0,
                start: 0.0,
                end: 0.0,
            });
            let serialized = serde_json::to_string(&(&canonical, &request.app_params))
                .map_err(|e| crate::error::ServerError::Cache(e.to_string()))?;
            let mut hasher = DefaultHasher::new();
            serialized.hash(&mut hasher);
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
                debug!("timeline chunk cache hit: {cache_key:?}");
                chunk_responses.push(cached);
                continue;
            }

            debug!("timeline chunk cache miss: {cache_key:?}");

            // Convert chunk span back to relative seconds for the request.
            let chunk_request = SingleTimelineRequest {
                entry: request.entry.clone().with_config(TimelineConfig {
                    num_bins,
                    start: to_secs_relative(chunk_start, epoch),
                    end: to_secs_relative(chunk_end, epoch),
                }),
                app_params: request.app_params.clone(),
            };

            let response = analyzer.single_resource_timeline(chunk_request)?;
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
