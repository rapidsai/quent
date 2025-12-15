"""
Tests for the Rust backend client.
"""
import pytest
from unittest.mock import Mock, patch
import requests
from fastapi import HTTPException

from webserver.client import RustBackendClient


@pytest.fixture
def client_instance():
    """Create a RustBackendClient instance for testing."""
    return RustBackendClient(base_url="http://test-backend:8080")


@pytest.mark.unit
def test_client_initialization(client_instance):
    """Test that client initializes correctly."""
    assert client_instance.base_url == "http://test-backend:8080"
    assert client_instance.timeout == 30


@pytest.mark.unit
@patch("webserver.client.requests.request")
def test_successful_get_request(mock_request, client_instance):
    """Test successful GET request."""
    mock_response = Mock()
    mock_response.json.return_value = {"data": "test"}
    mock_response.raise_for_status = Mock()
    mock_request.return_value = mock_response

    result = client_instance.get("/test", params={"key": "value"})

    assert result == {"data": "test"}
    mock_request.assert_called_once_with(
        method="GET",
        url="http://test-backend:8080/test",
        params={"key": "value"},
        json=None,
        timeout=30,
    )


@pytest.mark.unit
@patch("webserver.client.requests.request")
def test_successful_post_request(mock_request, client_instance):
    """Test successful POST request."""
    mock_response = Mock()
    mock_response.json.return_value = {"created": True}
    mock_response.raise_for_status = Mock()
    mock_request.return_value = mock_response

    result = client_instance.post("/test", json={"data": "test"})

    assert result == {"created": True}
    mock_request.assert_called_once_with(
        method="POST",
        url="http://test-backend:8080/test",
        params=None,
        json={"data": "test"},
        timeout=30,
    )


@pytest.mark.unit
@patch("webserver.client.requests.request")
def test_timeout_error(mock_request, client_instance):
    """Test handling of timeout errors."""
    mock_request.side_effect = requests.exceptions.Timeout()

    with pytest.raises(HTTPException) as exc_info:
        client_instance.get("/test")

    assert exc_info.value.status_code == 504
    assert "timed out" in exc_info.value.detail


@pytest.mark.unit
@patch("webserver.client.requests.request")
def test_connection_error(mock_request, client_instance):
    """Test handling of connection errors."""
    mock_request.side_effect = requests.exceptions.ConnectionError()

    with pytest.raises(HTTPException) as exc_info:
        client_instance.get("/test")

    assert exc_info.value.status_code == 503
    assert "unavailable" in exc_info.value.detail


@pytest.mark.unit
@patch("webserver.client.requests.request")
def test_http_error(mock_request, client_instance):
    """Test handling of HTTP errors."""
    mock_response = Mock()
    mock_response.status_code = 404
    mock_response.text = "Not Found"
    http_error = requests.exceptions.HTTPError(response=mock_response)
    http_error.response = mock_response

    mock_request.return_value = Mock()
    mock_request.return_value.raise_for_status.side_effect = http_error

    with pytest.raises(HTTPException) as exc_info:
        client_instance.get("/test")

    assert exc_info.value.status_code == 404
    assert "Not Found" in exc_info.value.detail


@pytest.mark.unit
@patch("webserver.client.requests.request")
def test_unexpected_error(mock_request, client_instance):
    """Test handling of unexpected errors."""
    mock_request.side_effect = Exception("Unexpected error")

    with pytest.raises(HTTPException) as exc_info:
        client_instance.get("/test")

    assert exc_info.value.status_code == 500
    assert "Internal server error" in exc_info.value.detail


@pytest.mark.unit
def test_base_url_trailing_slash_removal():
    """Test that trailing slashes are removed from base URL."""
    client = RustBackendClient(base_url="http://test:8080/")
    assert client.base_url == "http://test:8080"
