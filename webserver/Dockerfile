# syntax=docker/dockerfile:1

FROM python:3.12-slim

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

WORKDIR /app

# Hatchling build requires dependency files and README to be copied here
COPY pyproject.toml uv.lock README.md ./

# Install dependencies including dev dependencies (for debugpy)
RUN uv sync --frozen

COPY webserver ./webserver
COPY start.sh ./start.sh
RUN chmod +x ./start.sh

EXPOSE 8000
EXPOSE 5678

# Run webserver with debugpy for development
# Set DEBUG_MODE=true to disable --reload for debugging
# -Xfrozen_modules=off prevents debugpy from missing breakpoints
CMD ["./start.sh"]
