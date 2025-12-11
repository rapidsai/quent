"""
Configuration module for the webserver.
Centralizes all configuration settings for easy management.
"""
import os
from typing import List


class Settings:
    """Application settings and configuration."""

    RUST_BACKEND_URL: str = os.getenv("RUST_BACKEND_URL", "http://localhost:8080")

    CORS_ORIGINS: List[str] = [
        "http://localhost:5173",
        "http://127.0.0.1:5173",
        "http://localhost:8000",
        "http://127.0.0.1:8000",
    ]

    API_PREFIX: str = "/api"
    REQUEST_TIMEOUT: int = 30

    SERVER_HOST: str = os.getenv("SERVER_HOST", "0.0.0.0")
    SERVER_PORT: int = int(os.getenv("SERVER_PORT", "8000"))


settings = Settings()
