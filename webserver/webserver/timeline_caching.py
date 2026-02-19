"""
Timeline data caching system using chunking strategy.

This module implements a tiling/chunking system similar to map tiles, where timeline
data is divided into cacheable chunks at different zoom levels. This allows:
- Efficient caching of timeline data at various granularities
- Parallel fetching of multiple chunks
- Smooth zooming and panning without re-fetching entire timelines
"""

import asyncio
import logging
from typing import Any

from fastapi import HTTPException

from .cache import timeline_cache
from .client import rust_client
from .constants import TARGET_CHUNKS_PER_VIEW

logger = logging.getLogger(__name__)


def determine_zoom_level(start_pct: float, end_pct: float) -> int:
    """
    Determine appropriate zoom level based on the percentage range being viewed.
    """
    view_range = end_pct - start_pct
    if view_range <= 0:
        return 1

    # Our formula comes out to 100/(view_range/2), simplified to 200/view_range
    # By default we fetch ~2 chunks, so divide view_range by 2
    # Each chunk will be 100/N, where N=num of chunks
    target_level = int((100 * TARGET_CHUNKS_PER_VIEW) / view_range)

    return max(1, min(10, target_level))


def get_zoom_level_chunks(zoom_level: int, num_bins: int = 200) -> list[dict]:
    """
    Generate chunk definitions for a given zoom level. Zoom level N divides
    the timeline into N equal chunks.

    Each chunk dict defines a cacheable segment of the full query timeline:
    - start/end: Percentage boundaries (0-100) relative to the full query
    - zoom_level: Which zoom level this chunk belongs to
    - chunk_index: Position of this chunk within the zoom level
    - num_bins: Number of bins to request from the analyzer for this chunk
    """
    chunk_size = 100.0 / zoom_level
    chunks = []

    for i in range(zoom_level):
        start = i * chunk_size
        # Ensure last chunk ends exactly at 100 to avoid float issues
        end = 100.0 if i == zoom_level - 1 else (i + 1) * chunk_size

        chunks.append({
            "start": start,
            "end": end,
            "zoom_level": zoom_level,
            "chunk_index": i,
            "num_bins": num_bins
        })

    return chunks


def find_overlapping_chunks(chunks: list[dict], start_pct: float, end_pct: float) -> list[dict]:
    """
    Find all chunks that overlap with the given percentage range.
    """
    overlapping_chunks = []

    for chunk in chunks:
        if chunk["start"] < end_pct and chunk["end"] > start_pct:
            overlapping_chunks.append(chunk)

    return overlapping_chunks


async def fetch_chunk_with_cache(
    cache_key: str,
    fetch_url: str,
    ttl_seconds: int = 3600
) -> Any:
    """
    Fetch a single chunk with caching support.
    """
    cached_chunk = timeline_cache.get(cache_key)
    if cached_chunk is not None:
        logger.debug("Cache hit: %s", cache_key)
        return cached_chunk

    logger.debug("Cache miss: %s", cache_key)
    chunk_data = await asyncio.to_thread(rust_client.get, fetch_url)

    timeline_cache.set(cache_key, chunk_data, ttl_seconds=ttl_seconds)

    return chunk_data


def combine_chunk_result_data(
    chunks: list[dict],
    start: float,
    end: float
) -> dict:
    """
    Combine multiple timeline chunk data into a single response.
    """
    if not chunks:
        raise ValueError("No chunks to combine")

    is_binned_by_state = 'BinnedByState' in chunks[0]

    chunk_data = []
    for chunk in chunks:
        if is_binned_by_state:
            chunk_data.append(chunk['BinnedByState'])
        else:
            chunk_data.append(chunk['Binned'])

    sorted_chunks = sorted(chunk_data, key=lambda c: c['config']['span']['start'])
    bin_duration = sorted_chunks[0]['config']['bin_duration']

    # Extract relevant data values from each chunk
    combined_values = {}
    total_bins = 0

    for chunk in sorted_chunks:
        config = chunk['config']
        chunk_start = config['span']['start']
        chunk_end = config['span']['end']

        # First, find area of users view that overlaps with chunk range
        if chunk_end <= start or chunk_start >= end:
            continue

        overlap_start = max(start, chunk_start)
        overlap_end = min(end, chunk_end)

        # Next, convert time overlap to data value indices for this chunk and clamp
        start_idx = int(round((overlap_start - chunk_start) / bin_duration))
        end_idx = int(round((overlap_end - chunk_start) / bin_duration))

        start_idx = max(0, min(start_idx, config['num_bins']))
        end_idx = max(0, min(end_idx, config['num_bins']))

        bins_to_extract = end_idx - start_idx
        total_bins += bins_to_extract

        # Finally, extract and store data values from this chunk
        if is_binned_by_state:
            for capacity_name, states_dict in chunk['capacities_states_values'].items():
                if capacity_name not in combined_values:
                    combined_values[capacity_name] = {}

                for state_name, values in states_dict.items():
                    if state_name not in combined_values[capacity_name]:
                        combined_values[capacity_name][state_name] = []

                    combined_values[capacity_name][state_name].extend(
                        values[start_idx:end_idx]
                    )
        else:
            for capacity_name, values in chunk['capacities_values'].items():
                if capacity_name not in combined_values:
                    combined_values[capacity_name] = []

                combined_values[capacity_name].extend(values[start_idx:end_idx])

    result_config = {
        'span': {
            'start': start,
            'end': end
        },
        'bin_duration': bin_duration,
        'num_bins': total_bins
    }

    if is_binned_by_state:
        result = {
            'config': result_config,
            'capacities_states_values': combined_values
        }
        return {'BinnedByState': result}
    else:
        result = {
            'config': result_config,
            'capacities_values': combined_values
        }
        return {'Binned': result}


async def _fetch_timeline_with_chunks(
    num_bins: int,
    start: float,
    end: float,
    duration: float,
    engine_id: str,
    query_id: str,
    resource_id: str,
    resource_type: str,  # "resource" or "resource_group"
    fsm_type_name: str | None = None,
    resource_type_name: str | None = None
) -> Any:
    """
    Fetch timeline data using chunking strategy.
    """
    if duration <= 0:
        raise HTTPException(status_code=400, detail=f"Invalid duration: {duration}")
    if end <= start:
        raise HTTPException(status_code=400, detail=f"Invalid range: start={start}, end={end}")

    start_pct = (start / duration) * 100
    end_pct = (end / duration) * 100

    zoom_level = determine_zoom_level(start_pct, end_pct)
    log_prefix = "resource_group" if resource_type == "resource_group" else "resource"
    logger.debug("[%s] View range: %.1f%% - %.1f%% | zoom level: %d", log_prefix, start_pct, end_pct, zoom_level)

    all_chunks = get_zoom_level_chunks(zoom_level, num_bins=num_bins)

    required_chunks = find_overlapping_chunks(all_chunks, start_pct, end_pct)
    chunk_ranges = [f"{c['start']:.1f}-{c['end']:.1f}%" for c in required_chunks]
    logger.debug("[%s] Fetching %d chunks: %s", log_prefix, len(required_chunks), chunk_ranges)

    tasks = []
    for chunk in required_chunks:
        # convert chunk percentage boundaries to absolute time values
        chunk_start_time = (chunk['start'] / 100.0) * duration
        chunk_end_time = (chunk['end'] / 100.0) * duration

        # Build query params based on resource type
        query_params = f"?num_bins={chunk['num_bins']}&start={chunk_start_time}&end={chunk_end_time}"
        if resource_type == "resource_group":
            query_params += f"&resource_type_name={resource_type_name}"
        if fsm_type_name:
            query_params += f"&fsm_type_name={fsm_type_name}"

        # Generate cache key for this chunk (different format for resource vs resource_group)
        if resource_type == "resource_group":
            cache_key = (
                f"chunk:resource_group:{engine_id}:{query_id}:{resource_id}:"
                f"z{zoom_level}:c{chunk['chunk_index']}:"
                f"{chunk_start_time:.3f}:{chunk_end_time:.3f}:{chunk['num_bins']}:{resource_type_name}:{fsm_type_name}"
            )
        else:
            cache_key = (
                f"chunk:resource:{engine_id}:{query_id}:{resource_id}:"
                f"z{zoom_level}:c{chunk['chunk_index']}:"
                f"{chunk_start_time:.3f}:{chunk_end_time:.3f}:{chunk['num_bins']}:{fsm_type_name}"
            )

        fetch_url = f"/analyzer/engine/{engine_id}/query/{query_id}/{resource_type}/{resource_id}/timeline{query_params}"
        logger.debug("[%s] Fetching chunk %d: %s", log_prefix, chunk['chunk_index'], query_params)

        task = fetch_chunk_with_cache(cache_key, fetch_url)
        tasks.append(task)

    result_data = await asyncio.gather(*tasks)

    combined_result = combine_chunk_result_data(result_data, start, end)

    combined_bins = combined_result.get('Binned', combined_result.get('BinnedByState', {})).get('config', {}).get('num_bins', 0)
    logger.debug(
        "[%s] Combined %d chunks for %s %s | range: %.9f - %.9f (%.2f%% - %.2f%%) | bins: %d",
        log_prefix, len(result_data), resource_type, resource_id, start, end, start_pct, end_pct, combined_bins
    )
    return combined_result


async def get_timeline_bins(
    num_bins: int,
    start: float,
    end: float,
    duration: float,
    engine_id: str,
    query_id: str,
    resource_id: str,
    fsm_type_name: str | None
) -> Any:
    return await _fetch_timeline_with_chunks(
        num_bins=num_bins,
        start=start,
        end=end,
        duration=duration,
        engine_id=engine_id,
        query_id=query_id,
        resource_id=resource_id,
        resource_type="resource",
        fsm_type_name=fsm_type_name
    )


async def get_timeline_bins_for_resource_group(
    num_bins: int,
    start: float,
    end: float,
    duration: float,
    engine_id: str,
    query_id: str,
    resource_group_id: str,
    resource_type_name: str,
    fsm_type_name: str | None
) -> Any:
    return await _fetch_timeline_with_chunks(
        num_bins=num_bins,
        start=start,
        end=end,
        duration=duration,
        engine_id=engine_id,
        query_id=query_id,
        resource_id=resource_group_id,
        resource_type="resource_group",
        fsm_type_name=fsm_type_name,
        resource_type_name=resource_type_name
    )
