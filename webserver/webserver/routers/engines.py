"""
Engine-related API routes.
Handles all endpoints related to engines, workers, query groups, and queries.
"""

from typing import Any

from fastapi import APIRouter, Path, Query

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


@router.get("/{engine_id}/query/{query_id}/resource/{resource_id}/timeline/agg/fsm")
async def get_resource_timeline_aggregated_by_fsm(
    engine_id: str = Path(..., description="The engine ID"),
    query_id: str = Path(..., description="The query ID"),
    resource_id: str = Path(..., description="The resource ID"),
    fsm_type_name: str = Query(..., description="FSM type name"),
    num_bins: int = Query(..., description="Number of bins for aggregation"),
    start: float = Query(
        ..., description="Start time in seconds relative to query start"
    ),
    end: float = Query(..., description="End time in seconds relative to query start"),
) -> Any:
    """
    Fetches aggregated FSM timeline for a given resource within a query context.
    """
    return rust_client.get(
        f"/analyzer/engine/{engine_id}/query/{query_id}/resource/{resource_id}/timeline/agg/fsm"
        f"?num_bins={num_bins}&fsm_type_name={fsm_type_name}&start={start}&end={end}"
    )


@router.get("/{engine_id}/query/{query_id}/resource/{resource_id}/timeline/agg/all")
async def get_resource_timeline_aggregated_all(
    engine_id: str = Path(..., description="The engine ID"),
    query_id: str = Path(..., description="The query ID"),
    resource_id: str = Path(..., description="The resource ID"),
    num_bins: int = Query(..., description="Number of bins for aggregation"),
    start: float = Query(
        ..., description="Start time in seconds relative to query start"
    ),
    end: float = Query(..., description="End time in seconds relative to query start"),
) -> Any:
    """
    Fetches aggregated timeline for a given resource over all fsm states within a query context.
    """
    return rust_client.get(
        f"/analyzer/engine/{engine_id}/query/{query_id}/resource/{resource_id}/timeline/agg/all"
        f"?num_bins={num_bins}&start={start}&end={end}"
    )
