"""
Engine-related API routes.
Handles all endpoints related to engines, workers, coordinators, and queries.
"""
from fastapi import APIRouter, Path
from typing import Any, Dict

from client import rust_client

router = APIRouter(prefix="/engine", tags=["engines"])


@router.get("/list")
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


@router.get("/{engine_id}/coordinators")
async def list_coordinators(
    engine_id: str = Path(..., description="The engine ID"),
) -> Any:
    """
    List all coordinators for a given engine.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}/list_coordinators")


@router.get("/{engine_id}/coordinators/{coordinator_id}")
async def get_coordinator(
    engine_id: str = Path(..., description="The engine ID"),
    coordinator_id: str = Path(..., description="The coordinator ID"),
) -> Any:
    """
    Get details for a specific coordinator.
    """
    return rust_client.get(
        f"/analyzer/engine/{engine_id}/coordinator/{coordinator_id}"
    )


@router.get("/{engine_id}/coordinators/{coordinator_id}/queries")
async def list_coordinator_queries(
    engine_id: str = Path(..., description="The engine ID"),
    coordinator_id: str = Path(..., description="The coordinator ID"),
) -> Any:
    """
    List all queries for a specific coordinator.
    """
    return rust_client.get(
        f"/analyzer/engine/{engine_id}/coordinator/{coordinator_id}/list_queries"
    )


@router.get("/{engine_id}/query/{query_id}")
async def get_query(
    engine_id: str = Path(..., description="The engine ID"),
    query_id: str = Path(..., description="The query ID"),
) -> Any:
    """
    Get details for a specific query.
    """
    return rust_client.get(f"/analyzer/engine/{engine_id}/query/{query_id}")
