# syntax=docker/dockerfile:1

FROM python:3.12-slim

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

WORKDIR /app

# Hatchling build requires dependency files and README to be copied here
COPY pyproject.toml uv.lock README.md ./

RUN uv sync --frozen --no-dev

COPY webserver ./webserver

EXPOSE 8000

# Run webserver
CMD ["uv", "run", "uvicorn", "webserver.main:app", "--host", "0.0.0.0", "--port", "8000"]
