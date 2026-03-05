use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};

use moka::future::Cache;
use quent_analyzer::Span;
use quent_query_engine_analyzer::{QueryEngineModel, ui::UiAnalyzer};
use quent_time::{SpanNanoSec, TimeNanoSec, to_nanosecs, to_secs_relative};
use quent_ui::timeline::{
    request::{SingleTimelineRequest, TimelineConfig},
    response::{
        ResourceTimeline, ResourceTimelineBinned, ResourceTimelineBinnedByState,
        SingleTimelineResponse,
    },
};
use serde::Serialize;
use tracing::debug;
use uuid::Uuid;

use crate::error::ServerResult;

/// Target number of chunks visible in the current view range.
const TARGET_CHUNKS_PER_VIEW: u64 = 2;
/// Maximum zoom level (number of chunks the timeline is divided into).
const MAX_ZOOM_LEVEL: u64 = 10;

/// Cache for timeline chunk responses.
#[derive(Clone)]
pub struct TimelineCache {
    chunks: Cache<String, SingleTimelineResponse>,
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
        let req_start = epoch + to_nanosecs(request.config.start);
        let req_end = epoch + to_nanosecs(request.config.end);
        let req_span = match SpanNanoSec::try_new(req_start, req_end) {
            Ok(span) => span,
            Err(_) => return Ok(analyzer.single_resource_timeline(request)?),
        };

        let num_bins = request.config.num_bins;
        let view_duration = req_span.duration();

        if view_duration == 0 {
            return Ok(analyzer.single_resource_timeline(request)?);
        }

        let zoom_level = determine_zoom_level(view_duration, engine_duration);
        let chunk_duration = engine_duration / zoom_level;

        // Hash the entry + app_params for cache key construction.
        let params_hash = {
            let serialized =
                serde_json::to_string(&(&request.entry, &request.app_params)).unwrap_or_default();
            let mut hasher = DefaultHasher::new();
            serialized.hash(&mut hasher);
            hasher.finish()
        };

        // Fetch overlapping chunks from cache or compute them.
        let mut chunk_responses: Vec<SingleTimelineResponse> = Vec::new();
        for chunk_idx in 0..zoom_level {
            let chunk_start = epoch + chunk_idx * chunk_duration;
            let chunk_end = if chunk_idx == zoom_level - 1 {
                engine_span.end()
            } else {
                epoch + (chunk_idx + 1) * chunk_duration
            };

            let chunk_span = SpanNanoSec::try_new(chunk_start, chunk_end)
                .expect("chunk boundaries are always valid");

            if !chunk_span.intersects(&req_span) {
                continue;
            }

            let cache_key =
                format!("{engine_id}:{params_hash:x}:z{zoom_level}:c{chunk_idx}:{num_bins}");

            if let Some(cached) = self.chunks.get(&cache_key).await {
                debug!("timeline chunk cache hit: {cache_key}");
                chunk_responses.push(cached);
                continue;
            }

            debug!("timeline chunk cache miss: {cache_key}");

            // Convert chunk span back to relative seconds for the request.
            let chunk_request = SingleTimelineRequest {
                config: TimelineConfig {
                    num_bins,
                    start: to_secs_relative(chunk_start, epoch),
                    end: to_secs_relative(chunk_end, epoch),
                },
                entry: request.entry.clone(),
                app_params: request.app_params.clone(),
            };

            let response = analyzer.single_resource_timeline(chunk_request)?;
            self.chunks.insert(cache_key, response.clone()).await;
            chunk_responses.push(response);
        }

        if chunk_responses.is_empty() {
            return Ok(analyzer.single_resource_timeline(request)?);
        }

        // Convert request back to relative seconds for combining.
        let req_start_s = to_secs_relative(req_start, epoch);
        let req_end_s = to_secs_relative(req_end, epoch);

        if chunk_responses.len() == 1 {
            let chunk = chunk_responses.into_iter().next().unwrap();
            if (chunk.config.span.start() - req_start_s).abs() < 1e-9
                && (chunk.config.span.end() - req_end_s).abs() < 1e-9
            {
                return Ok(chunk);
            }
            return Ok(combine_chunks(&[chunk], req_start_s, req_end_s));
        }

        Ok(combine_chunks(&chunk_responses, req_start_s, req_end_s))
    }
}

fn determine_zoom_level(view_duration: TimeNanoSec, total_duration: TimeNanoSec) -> u64 {
    if view_duration == 0 {
        return 1;
    }
    let target = (total_duration * TARGET_CHUNKS_PER_VIEW) / view_duration;
    target.clamp(1, MAX_ZOOM_LEVEL)
}

fn combine_chunks(
    chunks: &[SingleTimelineResponse],
    req_start: f64,
    req_end: f64,
) -> SingleTimelineResponse {
    let mut sorted: Vec<&SingleTimelineResponse> = chunks.iter().collect();
    sorted.sort_by(|a, b| {
        a.config
            .span
            .start()
            .partial_cmp(&b.config.span.start())
            .unwrap()
    });

    let bin_duration = sorted[0].config.bin_duration;
    let is_binned_by_state = matches!(&sorted[0].data, ResourceTimeline::BinnedByState(_));

    if is_binned_by_state {
        let mut combined: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<f64>>,
        > = std::collections::HashMap::new();
        let mut total_bins: u64 = 0;

        for chunk in &sorted {
            let (start_idx, end_idx) = overlap_indices(chunk, req_start, req_end);
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

        SingleTimelineResponse {
            config: quent_time::bin::BinnedSpanSec {
                span: quent_time::span::SpanSec::new(req_start, req_end),
                bin_duration,
                num_bins: total_bins,
            },
            data: ResourceTimeline::BinnedByState(ResourceTimelineBinnedByState {
                capacities_states_values: combined,
                long_fsms: Vec::new(),
            }),
        }
    } else {
        let mut combined: std::collections::HashMap<String, Vec<f64>> =
            std::collections::HashMap::new();
        let mut total_bins: u64 = 0;

        for chunk in &sorted {
            let (start_idx, end_idx) = overlap_indices(chunk, req_start, req_end);
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

        SingleTimelineResponse {
            config: quent_time::bin::BinnedSpanSec {
                span: quent_time::span::SpanSec::new(req_start, req_end),
                bin_duration,
                num_bins: total_bins,
            },
            data: ResourceTimeline::Binned(ResourceTimelineBinned {
                capacities_values: combined,
                long_fsms: Vec::new(),
            }),
        }
    }
}

fn overlap_indices(chunk: &SingleTimelineResponse, req_start: f64, req_end: f64) -> (usize, usize) {
    let chunk_start = chunk.config.span.start();
    let chunk_end = chunk.config.span.end();
    let bin_duration = chunk.config.bin_duration;
    let num_bins = chunk.config.num_bins as usize;

    if chunk_end <= req_start || chunk_start >= req_end || bin_duration <= 0.0 {
        return (0, 0);
    }

    let overlap_start = req_start.max(chunk_start);
    let overlap_end = req_end.min(chunk_end);

    let start_idx = ((overlap_start - chunk_start) / bin_duration).round() as usize;
    let end_idx = ((overlap_end - chunk_start) / bin_duration).round() as usize;

    (start_idx.min(num_bins), end_idx.min(num_bins))
}
