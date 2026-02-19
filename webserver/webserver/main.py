"""
Main application entry point.
Sets up FastAPI app with middleware and routers.
"""
import logging

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from .config import settings
from .routers import engines

# Configure application logging so that logger.debug() etc. are visible.
# Uvicorn's --log-level only affects uvicorn's logs; app loggers use the root logger.
_level = getattr(logging, settings.LOG_LEVEL, logging.INFO)
logging.basicConfig(
    level=_level,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)

app = FastAPI(
    title="Quent Webserver",
    description="Proxy layer between Rust backend and React frontend",
    version="0.1.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.CORS_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(engines.router, prefix=settings.API_PREFIX)


@app.get("/")
async def read_root():
    """Health check endpoint."""
    return {
        "message": "Quent Webserver is running",
        "version": "0.1.0",
        "backend": settings.QUENT_ANALYZER_ADDRESS,
    }


@app.get("/health")
async def health_check():
    """Health check endpoint for monitoring."""
    return {"status": "healthy"}
