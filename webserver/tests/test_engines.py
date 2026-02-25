"""
Tests for engine-related API endpoints.
"""
import pytest
from fastapi import status
from unittest.mock import Mock, AsyncMock, patch


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
def test_list_query_group_queries(client, mock_rust_client, sample_query_data):
    """Test listing queries for a query group."""
    engine_id = "engine-1"
    query_group_id = "qg-1"
    mock_rust_client.get.return_value = [sample_query_data]

    response = client.get(
        f"/api/engines/{engine_id}/query_group/{query_group_id}/queries"
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


@pytest.mark.unit
def test_get_resource_group_timeline(client):
    engine_id = "engine-1"
    query_id = "query-1"
    resource_group_id = "rg-1"

    mock_result = {
        "Binned": {
            "config": {
                "span": {"start": 0.0, "end": 0.523},
                "bin_duration": 0.00261,
                "num_bins": 200,
            },
            "capacities_values": {"thread": [0.0] * 200},
        }
    }

    with patch("webserver.routers.engines.get_timeline_bins_for_resource_group",
new_callable=AsyncMock) as mock_fn:
        mock_fn.return_value = mock_result

        response = client.get(
            f"/api/engines/{engine_id}/query/{query_id}/resource_group/{resource_group_id}/timeline",
            params={
                "num_bins": 200,
                "start": 0.0,
                "end": 0.523309945,
                "duration": 0.523309945,
                "resource_type_name": "thread",
            },
        )

    assert response.status_code == status.HTTP_200_OK
    assert response.json() == mock_result
    mock_fn.assert_called_once_with(
        200, 0.0, 0.523309945, 0.523309945,
        engine_id, query_id, resource_group_id, "thread", None
    )


@pytest.mark.unit
def test_get_resource_timeline(client):
    engine_id = "engine-1"
    query_id = "query-1"
    resource_id = "res-1"

    mock_result = {
        "BinnedByState": {
            "config": {
                "span": {"start": 0.0, "end": 0.523},
                "bin_duration": 0.00261,
                "num_bins": 200,
            },
            "capacities_states_values": {"cpu": {"running": [0.0] * 200}},
        }
    }

    with patch("webserver.routers.engines.get_timeline_bins", new_callable=AsyncMock) as mock_fn:
        mock_fn.return_value = mock_result

        response = client.get(f"/api/engines/{engine_id}/query/{query_id}/resource/{resource_id}/timeline",
            params={
                "num_bins": 200,
                "start": 0.0,
                "end": 0.523309945,
                "duration": 0.523309945,
            },
        )

    assert response.status_code == status.HTTP_200_OK
    assert response.json() == mock_result
    mock_fn.assert_called_once_with(
        200, 0.0, 0.523309945, 0.523309945,
        engine_id, query_id, resource_id, None
    )
