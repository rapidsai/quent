"""
Pytest configuration and shared fixtures for tests.
"""
import pytest
from fastapi.testclient import TestClient
from unittest.mock import Mock, patch

from webserver.main import app
from webserver.client import RustBackendClient


@pytest.fixture
def client():
    """
    FastAPI test client fixture.
    Provides a test client for making requests to the API.
    """
    return TestClient(app)


@pytest.fixture
def mock_rust_client(monkeypatch):
    """
    Mock the Rust backend client to avoid real HTTP calls during tests.
    Returns a mock client that can be configured per test.
    """
    mock = Mock(spec=RustBackendClient)

    # Patch the rust_client singleton
    monkeypatch.setattr("webserver.routers.engines.rust_client", mock)

    return mock


@pytest.fixture
def sample_engine_data():
    """Sample engine data for testing."""
    return {
        "id": "engine-1",
        "name": "Test Engine",
        "status": "running",
        "workers": 5
    }


@pytest.fixture
def sample_worker_data():
    """Sample worker data for testing."""
    return {
        "id": "worker-1",
        "engine_id": "engine-1",
        "status": "active",
        "queries": 10
    }


@pytest.fixture
def sample_query_group_data():
    """Sample query group data for testing."""
    return {
        "id": "qg-1",
        "engine_id": "engine-1",
        "name": "Test Query Group",
        "query_count": 3
    }


@pytest.fixture
def sample_query_data():
    """Sample query data for testing."""
    return {
        "id": "query-1",
        "engine_id": "engine-1",
        "query_group_id": "qg-1",
        "sql": "SELECT * FROM test",
        "plan": {"nodes": []}
    }
