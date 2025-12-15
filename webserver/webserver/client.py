"""
Client module for communicating with the Rust backend service.
Provides a centralized, reusable HTTP client with error handling.
"""
import logging
from typing import Any, Dict, Optional

import requests
from fastapi import HTTPException

from .config import settings

logger = logging.getLogger(__name__)


class RustBackendClient:
    """HTTP client for communicating with the Rust backend service."""

    def __init__(self, base_url: str = settings.RUST_BACKEND_URL):
        self.base_url = base_url.rstrip("/")
        self.timeout = settings.REQUEST_TIMEOUT

    def _make_request(
        self,
        method: str,
        path: str,
        params: Optional[Dict[str, Any]] = None,
        json: Optional[Dict[str, Any]] = None,
    ) -> Any:
        """
        Make an HTTP request to the Rust backend.

        Args:
            method: HTTP method (GET, POST, etc.)
            path: API path (should start with /)
            params: Query parameters
            json: JSON body for POST/PUT requests

        Returns:
            Response JSON data

        Raises:
            HTTPException: If the request fails
        """
        url = f"{self.base_url}{path}"
        logger.info(f"{method} {url}")

        try:
            response = requests.request(
                method=method,
                url=url,
                params=params,
                json=json,
                timeout=self.timeout,
            )
            response.raise_for_status()
            return response.json()

        except requests.exceptions.Timeout:
            logger.error(f"Request timeout: {url}")
            raise HTTPException(
                status_code=504,
                detail="Backend service request timed out",
            )

        except requests.exceptions.ConnectionError:
            logger.error(f"Connection error: {url}")
            raise HTTPException(
                status_code=503,
                detail="Backend service unavailable",
            )

        except requests.exceptions.HTTPError as e:
            logger.error(f"HTTP error: {e}")
            raise HTTPException(
                status_code=e.response.status_code,
                detail=e.response.text or "Backend request failed",
            )

        except Exception as e:
            logger.error(f"Unexpected error: {e}")
            raise HTTPException(
                status_code=500,
                detail="Internal server error",
            )

    def get(self, path: str, params: Optional[Dict[str, Any]] = None) -> Any:
        return self._make_request("GET", path, params=params)

    def post(self, path: str, json: Optional[Dict[str, Any]] = None) -> Any:
        return self._make_request("POST", path, json=json)

    def put(self, path: str, json: Optional[Dict[str, Any]] = None) -> Any:
        return self._make_request("PUT", path, json=json)

    def delete(self, path: str) -> Any:
        return self._make_request("DELETE", path)


rust_client = RustBackendClient()
