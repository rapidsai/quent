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


@router.get("/{engine_id}/query/{query_id}/resource/{resource_id}/timeline")
async def get_resource_timeline(
    engine_id: str = Path(..., description="The engine ID"),
    query_id: str = Path(..., description="The query ID"),
    resource_id: str = Path(..., description="The resource ID"),
    num_bins: int = Query(..., description="Number of bins for aggregation"),
    start: float = Query(
        ..., description="Start time in seconds relative to query start"
    ),
    end: float = Query(..., description="End time in seconds relative to query start"),
    fsm_type_name: str | None = Query(
        None,
        description="Optional FSM type name to aggregate by state. If not provided, aggregates across all states.",
    ),
) -> Any:
    """
    Fetches timeline of utilization of a single resource.
    Returns bins in which utilization is aggregated across all FSM states, or per state if fsm_type_name is provided.
    """
    query_params = f"?num_bins={num_bins}&start={start}&end={end}"
    if fsm_type_name:
        query_params += f"&fsm_type_name={fsm_type_name}"
    return rust_client.get(
        f"/analyzer/engine/{engine_id}/query/{query_id}/resource/{resource_id}/timeline{query_params}"
    )


@router.get("/{engine_id}/query/{query_id}/resource_group/{resource_group_id}/timeline")
async def get_resource_group_timeline(
    engine_id: str = Path(..., description="The engine ID"),
    query_id: str = Path(..., description="The query ID"),
    resource_group_id: str = Path(..., description="The resource group ID"),
    num_bins: int = Query(..., description="Number of bins for aggregation"),
    start: float = Query(
        ..., description="Start time in seconds relative to query start"
    ),
    end: float = Query(..., description="End time in seconds relative to query start"),
    resource_type_name: str = Query(
        ..., description="Resource type name for aggregation"
    ),
    fsm_type_name: str | None = Query(
        None,
        description="Optional FSM type name to aggregate by state. If not provided, aggregates across all states.",
    ),
) -> Any:
    """
    Fetches timeline resource utilization of all resource with the same type under a resource group.
    Returns bins in which utilization is aggregated across all FSM states, or per state if fsm_type_name is provided.
    """
    query_params = f"?num_bins={num_bins}&start={start}&end={end}&resource_type_name={resource_type_name}"
    if fsm_type_name:
        query_params += f"&fsm_type_name={fsm_type_name}"
    return rust_client.get(
        f"/analyzer/engine/{engine_id}/query/{query_id}/resource_group/{resource_group_id}/timeline{query_params}"
    )
