"""
Tests for main application endpoints.
"""
import pytest
from fastapi import status


@pytest.mark.unit
def test_read_root(client):
    """Test the root endpoint returns expected data."""
    response = client.get("/")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert "message" in data
    assert data["message"] == "Quent Webserver is running"
    assert "version" in data
    assert data["version"] == "0.1.0"
    assert "backend" in data


@pytest.mark.unit
def test_health_check(client):
    """Test the health check endpoint."""
    response = client.get("/health")

    assert response.status_code == status.HTTP_200_OK
    data = response.json()
    assert data == {"status": "healthy"}


@pytest.mark.unit
def test_cors_headers(client):
    """Test that CORS headers are properly configured."""
    response = client.options(
        "/",
        headers={
            "Origin": "http://localhost:5173",
            "Access-Control-Request-Method": "GET",
        }
    )

    assert response.status_code == status.HTTP_200_OK
    assert "access-control-allow-origin" in response.headers
