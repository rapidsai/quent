#!/bin/sh

if [ -z "$LOG_LEVEL" ]; then
    LOG_LEVEL=INFO
fi
export LOG_LEVEL

# Conditionally enable uvicorn --reload based on DEBUG_MODE env var
if [ "$DEBUG_MODE" = "true" ]; then
    echo "Starting in DEBUG mode (no reload)..."
    exec uv run python -Xfrozen_modules=off -m debugpy --listen 0.0.0.0:5678 -m uvicorn webserver.main:app --host 0.0.0.0 --port 8000
else
    echo "Starting in DEVELOPMENT mode (with reload)..."
    exec uv run python -Xfrozen_modules=off -m debugpy --listen 0.0.0.0:5678 -m uvicorn webserver.main:app --host 0.0.0.0 --port 8000 --reload
fi
