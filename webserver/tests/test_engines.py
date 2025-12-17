"""
Tests for engine-related API endpoints.
"""
import pytest
from fastapi import status
from unittest.mock import Mock


@pytest.mark.unit
def test_list_engines(client, mock_rust_client, sample_engine_data):
    """Test listing all engines."""
    mock_rust_client.get.return_value = [sample_engine_data]

    response = client.get("/api/engines/")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert isinstance(data, list)
    assert len(data) == 1
    assert data[0]["id"] == "engine-1"
    mock_rust_client.get.assert_called_once_with("/analyzer/list_engines")


@pytest.mark.unit
def test_get_engine(client, mock_rust_client, sample_engine_data):
    """Test getting a specific engine."""
    engine_id = "engine-1"
    mock_rust_client.get.return_value = sample_engine_data

    response = client.get(f"/api/engines/{engine_id}")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert data["id"] == engine_id
    mock_rust_client.get.assert_called_once_with(f"/analyzer/engine/{engine_id}")


@pytest.mark.unit
def test_list_workers(client, mock_rust_client, sample_worker_data):
    """Test listing workers for an engine."""
    engine_id = "engine-1"
    mock_rust_client.get.return_value = [sample_worker_data]

    response = client.get(f"/api/engines/{engine_id}/workers")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert isinstance(data, list)
    assert len(data) == 1
    mock_rust_client.get.assert_called_once_with(
        f"/analyzer/engine/{engine_id}/list_workers"
    )


@pytest.mark.unit
def test_get_worker(client, mock_rust_client, sample_worker_data):
    """Test getting a specific worker."""
    engine_id = "engine-1"
    worker_id = "worker-1"
    mock_rust_client.get.return_value = sample_worker_data

    response = client.get(f"/api/engines/{engine_id}/workers/{worker_id}")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert data["id"] == worker_id
    mock_rust_client.get.assert_called_once_with(
        f"/analyzer/engine/{engine_id}/worker/{worker_id}"
    )


@pytest.mark.unit
def test_list_query_groups(client, mock_rust_client, sample_query_group_data):
    """Test listing query groups for an engine."""
    engine_id = "engine-1"
    mock_rust_client.get.return_value = [sample_query_group_data]

    response = client.get(f"/api/engines/{engine_id}/query-groups")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert isinstance(data, list)
    mock_rust_client.get.assert_called_once_with(
        f"/analyzer/engine/{engine_id}/list_query_groups"
    )


@pytest.mark.unit
def test_get_query_group(client, mock_rust_client, sample_query_group_data):
    """Test getting a specific query group."""
    engine_id = "engine-1"
    query_group_id = "qg-1"
    mock_rust_client.get.return_value = sample_query_group_data

    response = client.get(f"/api/engines/{engine_id}/query-groups/{query_group_id}")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert data["id"] == query_group_id
    mock_rust_client.get.assert_called_once_with(
        f"/analyzer/engine/{engine_id}/query_group/{query_group_id}"
    )


@pytest.mark.unit
def test_list_query_group_queries(client, mock_rust_client, sample_query_data):
    """Test listing queries for a query group."""
    engine_id = "engine-1"
    query_group_id = "qg-1"
    mock_rust_client.get.return_value = [sample_query_data]

    response = client.get(
        f"/api/engines/{engine_id}/query-groups/{query_group_id}/queries"
    )

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert isinstance(data, list)
    mock_rust_client.get.assert_called_once_with(
        f"/analyzer/engine/{engine_id}/query_group/{query_group_id}/list_queries"
    )


@pytest.mark.unit
def test_get_query(client, mock_rust_client, sample_query_data):
    """Test getting a specific query."""
    engine_id = "engine-1"
    query_id = "query-1"
    mock_rust_client.get.return_value = sample_query_data

    response = client.get(f"/api/engines/{engine_id}/query/{query_id}")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert data["id"] == query_id
    mock_rust_client.get.assert_called_once_with(
        f"/analyzer/engine/{engine_id}/query/{query_id}"
    )
