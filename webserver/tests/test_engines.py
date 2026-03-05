"""
Tests for engine-related API endpoints.
"""
import pytest
from fastapi import status
from unittest.mock import patch, AsyncMock


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
    mock_rust_client.get.assert_called_once_with("/analyzer/list_engines", params={"with_metadata": "false"})


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
def test_get_single_timeline_resource(client):
    """Test POST /timeline/single dispatches to get_timeline_bins for a Resource entry."""
    engine_id = "engine-1"
    request_body = {
        "config": {"num_bins": 200, "start": 0.0, "end": 10.0},
        "entry": {
            "Resource": {
                "resource_id": "res-1",
                "long_entities_threshold_s": None,
                "entity_filter": {"entity_type_name": "MyFsm"},
                "application": {"operator_id": None},
            }
        },
        "app_params": {"query_id": "query-1"},
    }
    expected_result = {"config": {"num_bins": 200, "bin_duration": 0.05, "span": {"start": 0.0, "end": 10.0}}, "data": {"Binned": {"capacities_values": {}, "long_fsms": []}}}

    with patch(
        "webserver.routers.engines.get_timeline_bins",
        new=AsyncMock(return_value=expected_result),
    ) as mock_get_bins:
        response = client.post(
            f"/api/engines/{engine_id}/timeline/single?duration=10.0",
            json=request_body,
        )

    assert response.status_code == status.HTTP_200_OK
    mock_get_bins.assert_called_once_with(
        num_bins=200,
        start=0.0,
        end=10.0,
        duration=10.0,
        engine_id=engine_id,
        query_id="query-1",
        resource_id="res-1",
        entity_type_name="MyFsm",
    )


@pytest.mark.unit
def test_get_bulk_timelines(client, mock_rust_client):
    """Test POST /timeline/bulk passes the request through to the Rust backend."""
    engine_id = "engine-1"
    request_body = {
        "config": {"num_bins": 200, "start": 0.0, "end": 10.0},
        "entries": {},
        "app_params": {"query_id": "query-1"},
    }
    expected_result = {"config": {}, "entries": {}}
    mock_rust_client.post.return_value = expected_result

    response = client.post(
        f"/api/engines/{engine_id}/timeline/bulk",
        json=request_body,
    )

    assert response.status_code == status.HTTP_200_OK
    mock_rust_client.post.assert_called_once_with(
        f"/analyzer/engine/{engine_id}/timeline/bulk",
        json=request_body,
    )
