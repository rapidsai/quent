#!/bin/bash

uv run uvicorn webserver.main:app --reload --port 8000
