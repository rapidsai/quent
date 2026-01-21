"""
Engine-related API routes.
Handles all endpoints related to engines, workers, query groups, and queries.
"""

from typing import Any

from fastapi import APIRouter, Path

from ..client import rust_client

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


@router.get("/{engine_id}/workers")
async def list_workers(
    engine_id: str = Path(..., description="The engine ID"),
) -> Any:
    """
    List all workers for a given engine.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}/list_workers")


@router.get("/{engine_id}/workers/{worker_id}")
async def get_worker(
    engine_id: str = Path(..., description="The engine ID"),
    worker_id: str = Path(..., description="The worker ID"),
) -> Any:
    """
    Get details for a given worker.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}/worker/{worker_id}")


@router.get("/{engine_id}/query-groups")
async def list_query_groups(
    engine_id: str = Path(..., description="The engine ID"),
) -> Any:
    """
    List all query_groups for a given engine.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}/list_query_groups")


@router.get("/{engine_id}/query-groups/{query_group_id}")
async def get_query_group(
    engine_id: str = Path(..., description="The engine ID"),
    query_group_id: str = Path(..., description="The query_group ID"),
) -> Any:
    """
    Get details for a specific query_group.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}/query_group/{query_group_id}")


@router.get("/{engine_id}/query-groups/{query_group_id}/queries")
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


@router.get("/{engine_id}/resource/{resource_id}/timeline/aggregated")
async def get_resource_timeline_aggregated(
    engine_id: str = Path(..., description="The engine ID"),
    resource_id: str = Path(..., description="The resource ID"),
    num_bins: int = 10,
    fsm_type_name: str = "task",
) -> Any:
    """
    Fetches aggregated FSM timeline for a given resource.
    """
    return rust_client.get(
        f"/analyzer/engine/{engine_id}/timeline/resource/{resource_id}/agg/fsm"
        f"?num_bins={num_bins}&fsm_type_name={fsm_type_name}"
    )