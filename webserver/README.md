# Quent Webserver

A FastAPI-based proxy layer that sits between the Rust backend services and the
React frontend application. This webserver initially proxies requests to the
backend, but will evolve to include data transformation and custom route
construction as the application grows.

## Overview

The webserver provides a clean HTTP API layer that:

- Proxies requests to Rust backend services
- Handles error translation and proper HTTP status codes
- Provides CORS support for the React frontend
- Will support data transformation and aggregation in the future

## Prerequisites

- Python 3.12 or higher
- [uv](https://docs.astral.sh/uv/) for dependency management
- Rust backend services running (by default running on `localhost:8080`)

## Installation

1. **Clone the repository**:

   ```bash
   cd ~/Projects/quent/webserver
   ```

2. **Install dependencies using uv**:

   ```bash
   uv sync
   ```

   This will create a virtual environment and install all required dependencies:
   - FastAPI (web framework)
   - Uvicorn (ASGI server)
   - Requests (HTTP client)
   - httpx (HTTP client for testing)

3. **Install development dependencies** (for testing):

   ```bash
   uv sync --all-extras
   ```

## Running the Server

### Quick Start

```bash
./start.sh
```

This will start the webserver on `http://localhost:8000` with auto-reload enabled.

### Manual Start

If you prefer to run manually:

```bash
uv run uvicorn webserver.main:app --reload --port 8000
```

### Verify It's Running

Visit `http://localhost:8000` in your browser. You should see:

```json
{
  "message": "Quent Webserver is running",
  "version": "0.1.0",
  "backend": "http://localhost:8080"
}
```

## Project Structure

```text
webserver/
  webserver/
    config.py         # Centralized configuration
    client.py         # HTTP client for Rust backend
    main.py           # FastAPI app entry point
    routers/
      __init__.py
      engines.py      # Engine-related routes
  tests/              # Test suite
    conftest.py       # Pytest fixtures
    test_main.py      # Tests for main endpoints
    test_engines.py   # Tests for engine routes
    test_client.py    # Tests for backend client
  pyproject.toml      # Project dependencies
  uv.lock             # Locked dependencies
  start.sh            # Convenience script to start server
  README.md           # This file
```

## Available Routes

### Health Check

- `GET /` - Server status and version
- `GET /health` - Health check endpoint

### Interactive API Documentation

FastAPI automatically generates interactive API documentation:

- **Swagger UI**: <http://localhost:8000/docs>
- **ReDoc**: <http://localhost:8000/redoc>

## Configuration

Configuration is centralized in `webserver/config.py`. You can override settings
using environment variables:

### Environment Variables

- `RUST_BACKEND_URL` - Rust backend URL (default: `http://localhost:8080`)
- `SERVER_HOST` - Host to bind to (default: `0.0.0.0`)
- `SERVER_PORT` - Port to bind to (default: `8000`)

### Example

```bash
RUST_BACKEND_URL=http://backend.example.com:9000 ./start.sh
```

### CORS Configuration

CORS is configured in `webserver/config.py` to allow the following origins:

- `http://localhost:5173` (Vite dev server)
- `http://127.0.0.1:5173`
- `http://localhost:8000`
- `http://127.0.0.1:8000`

Add additional origins to the `CORS_ORIGINS` list as needed.

## Development

### Adding New Routes

1. **Create or update a router** in `webserver/routers/`:

   ```python
   # webserver/routers/my_router.py
   from fastapi import APIRouter
   from ..client import rust_client

   router = APIRouter(prefix="/my-resource", tags=["my-resource"])

   @router.get("/{id}")
   async def get_item(id: str):
       return rust_client.get(f"/analyzer/my-resource/{id}")
   ```

2. **Register the router** in `webserver/main.py`:

   ```python
   from .routers import engines, my_router

   app.include_router(engines.router, prefix=settings.API_PREFIX)
   app.include_router(my_router.router, prefix=settings.API_PREFIX)
   ```

### Error Handling

The HTTP client (`webserver/client.py`) automatically handles common errors:

- **Timeout**  504 Gateway Timeout
- **Connection Error**  503 Service Unavailable
- **HTTP Errors**  Forwards status code from backend
- **Unexpected Errors**  500 Internal Server Error

### Debugging

To see detailed logs, configure logging in `webserver/main.py`:

```python
import logging

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
```

Or use print statements for quick debugging:

```python
print(f"Debug: {variable}")  # Will show in terminal
```

## Testing

The project uses pytest for testing with comprehensive test coverage.

### Running Tests

```bash
# Run all tests
uv run pytest

# Run with verbose output
uv run pytest -v

# Run specific test file
uv run pytest tests/test_main.py

# Run tests matching a pattern
uv run pytest -k "test_engine"

# Run only unit tests
uv run pytest -m unit

# Run with coverage report
uv run pytest --cov=webserver --cov-report=html

# Run tests in parallel (if you install pytest-xdist)
uv run pytest -n auto
```

### Writing Tests

Tests use pytest fixtures from `conftest.py`:

```python
def test_my_endpoint(client, mock_rust_client):
    """Test example using fixtures."""
    # Configure mock response
    mock_rust_client.get.return_value = {"data": "test"}

    # Make request
    response = client.get("/api/engine/list")

    # Assert results
    assert response.status_code == 200
    assert response.json() == {"data": "test"}
    mock_rust_client.get.assert_called_once()
```

### Test Markers

- `@pytest.mark.unit` - Fast unit tests (default)
- `@pytest.mark.integration` - Integration tests requiring external services
- `@pytest.mark.slow` - Slow-running tests

### Coverage Reports

After running tests with `--cov`, view the HTML coverage report:

```bash
# Generate and open coverage report
uv run pytest --cov=webserver --cov-report=html
xdg-open htmlcov/index.html  # Linux
open htmlcov/index.html  # macOS
```

## License

TDB
