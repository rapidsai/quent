#!/bin/bash

PYTHONPATH=src poetry run uvicorn src.main:app --reload --port 8000
