"""
Configuration module for the webserver.
Centralizes all configuration settings for easy management.
"""
import os
from typing import List


class Settings:
    """Application settings and configuration."""

    QUENT_ANALYZER_ADDRESS: str = os.getenv("QUENT_ANALYZER_ADDRESS", "http://localhost:8080")

    CORS_ORIGINS: List[str] = [
        "http://localhost:5173",
        "http://127.0.0.1:5173",
        "http://localhost:4173",
        "http://127.0.0.1:4173",
        "http://localhost:8000",
        "http://127.0.0.1:8000",
    ]

    API_PREFIX: str = "/api"
    REQUEST_TIMEOUT: int = 30

    SERVER_HOST: str = os.getenv("SERVER_HOST", "0.0.0.0")
    SERVER_PORT: int = int(os.getenv("SERVER_PORT", "8000"))

    # Application log level (DEBUG, INFO, WARNING, ERROR). Used by Python logging module.
    LOG_LEVEL: str = os.getenv("LOG_LEVEL", "INFO").upper()


settings = Settings()
