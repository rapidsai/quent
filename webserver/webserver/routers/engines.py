"""
Engine-related API routes.
Handles all endpoints related to engines, workers, query groups, and queries.
"""

from typing import Any

from fastapi import APIRouter, Body, HTTPException, Path, Query

from ..client import rust_client
from ..timeline_caching import (
    get_timeline_bins,
    get_timeline_bins_for_resource_group,
)

router = APIRouter(prefix="/engines", tags=["engines"])


@router.get("/")
async def list_engines() -> Any:
    """
    List all available engines.
    """
    return rust_client.get("/analyzer/list_engines")


@router.get("/{engine_id}")
async def get_engine(
    engine_id: str = Path(..., description="The engine ID"),
) -> Any:
    """
    Get details for a specific engine.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}")


@router.get("/{engine_id}/query-groups")
async def list_query_groups(
    engine_id: str = Path(..., description="The engine ID"),
) -> Any:
    """
    List all query_groups for a given engine.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}/list_query_groups")


@router.get("/{engine_id}/query_group/{query_group_id}/queries")
async def list_query_group_queries(
    engine_id: str = Path(..., description="The engine ID"),
    query_group_id: str = Path(..., description="The query_group ID"),
) -> Any:
    """
    List all queries for a specific query_group.
    """
    return rust_client.get(
        f"/analyzer/engine/{engine_id}/query_group/{query_group_id}/list_queries"
    )


@router.get("/{engine_id}/query/{query_id}")
async def get_query(
    engine_id: str = Path(..., description="The engine ID"),
    query_id: str = Path(..., description="The query ID"),
) -> Any:
    """
    Fetches query plan for given query.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}/query/{query_id}")


@router.post("/{engine_id}/timeline/single")
async def get_single_timeline(
    request: Any = Body(...),
    engine_id: str = Path(..., description="The engine ID"),
    duration: float = Query(..., description="Total query duration in seconds (for cache chunking)"),
) -> Any:
    """
    Fetches a single resource or resource-group timeline with chunk-based caching.
    """
    config = request['config']
    query_id = request['app_params']['query_id']
    entry = request['entry']

    if 'Resource' in entry:
        resource = entry['Resource']
        return await get_timeline_bins(
            num_bins=config['num_bins'],
            start=config['start'],
            end=config['end'],
            duration=duration,
            engine_id=engine_id,
            query_id=query_id,
            resource_id=resource['resource_id'],
            entity_type_name=resource['entity_filter']['entity_type_name'],
        )
    elif 'ResourceGroup' in entry:
        resource_group = entry['ResourceGroup']
        return await get_timeline_bins_for_resource_group(
            num_bins=config['num_bins'],
            start=config['start'],
            end=config['end'],
            duration=duration,
            engine_id=engine_id,
            query_id=query_id,
            resource_group_id=resource_group['resource_group_id'],
            resource_type_name=resource_group['resource_type_name'],
            entity_type_name=resource_group['entity_filter']['entity_type_name'],
        )
    else:
        raise HTTPException(status_code=400, detail="Unknown entry type in timeline request")


@router.post("/{engine_id}/timeline/bulk")
async def get_bulk_timelines(
    request: Any = Body(...),
    engine_id: str = Path(..., description="The engine ID"),
) -> Any:
    """
    Fetches multiple resource/resource-group timelines in one request (passthrough, no caching).
    """
    return rust_client.post(
        f"/analyzer/engine/{engine_id}/timeline/bulk",
        json=request,
    )
