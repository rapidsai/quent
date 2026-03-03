"""
Main application entry point.
Sets up FastAPI app with middleware and routers.
"""
import logging
from pathlib import Path

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

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


@app.get("/health")
async def health_check():
    """Health check endpoint for monitoring."""
    return {"status": "healthy"}


# Serve built UI assets if the static directory exists (i.e. inside Docker).
# /assets is served directly by StaticFiles. For all other non-API paths that
# would 404, the middleware serves index.html for SPA client-side routing.
_static = Path(settings.STATIC_DIR)
if _static.is_dir():
    app.mount("/assets", StaticFiles(directory=_static / "assets"), name="static-assets")

    _index_html = _static / "index.html"

    @app.middleware("http")
    async def spa_fallback(request, call_next):
        response = await call_next(request)
        if (
            response.status_code == 404
            and not request.url.path.startswith("/api")
            and not request.url.path.startswith("/health")
        ):
            return FileResponse(_index_html)
        return response
