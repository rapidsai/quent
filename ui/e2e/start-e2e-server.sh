#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DATA_DIR="${PLAYWRIGHT_E2E_DATA_DIR:-$SCRIPT_DIR/../.e2e-data}"
ANALYZER_URL="${PLAYWRIGHT_API_BASE_URL:-http://127.0.0.1:18080}"
COLLECTOR_ADDRESS="${PLAYWRIGHT_COLLECTOR_ADDRESS:-127.0.0.1:17836}"
COLLECTOR_URL="${PLAYWRIGHT_COLLECTOR_URL:-http://$COLLECTOR_ADDRESS}"
ENGINE_ID="00000000-0000-0000-0000-000000000001"
QUERY_GROUP_ID="00000000-0000-0000-0000-000000000003"
QUERY_ID="00000000-0000-0000-0000-000000000004"

read -ra CARGO <<< "${PLAYWRIGHT_CARGO_COMMAND:-cargo}"

# Strip scheme and trailing path to get host:port
ANALYZER_HOST="${ANALYZER_URL#*://}"
ANALYZER_HOST="${ANALYZER_HOST%%/*}"

rm -rf "$DATA_DIR"
mkdir -p "$DATA_DIR"

"${CARGO[@]}" run -p quent-simulator-server -- \
  --log-level warn \
  --collector-address "$COLLECTOR_ADDRESS" \
  --analyzer-address "$ANALYZER_HOST" \
  --cors-address "http://127.0.0.1:5173" \
  --output-dir "$DATA_DIR" \
  > "$DATA_DIR/server.log" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$DATA_DIR/server.pid"

wait_for_url() {
  local url="$1"
  local deadline=$(( $(date +%s) + 120 ))
  until curl -sf "$url" > /dev/null 2>&1; do
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
      echo "quent-simulator-server (pid $SERVER_PID) exited before becoming ready" >&2
      exit 1
    fi
    (( $(date +%s) < deadline )) || { echo "Timed out waiting for $url" >&2; exit 1; }
    sleep 0.5
  done
}

wait_for_record() {
  local url="$1" id="$2" name="$3"
  local deadline=$(( $(date +%s) + 120 ))
  while true; do
    local body
    body=$(curl -sf "$url" 2>/dev/null || true)
    if echo "$body" | grep -q "\"$id\"" && echo "$body" | grep -q "\"$name\""; then
      return 0
    fi
    (( $(date +%s) < deadline )) || { echo "Timed out waiting for $name at $url" >&2; exit 1; }
    sleep 0.5
  done
}

wait_for_url "$ANALYZER_URL/api/engines"

(cd "$REPO_ROOT" && "${CARGO[@]}" run -p quent-query-engine-fixed -- --collector-address "$COLLECTOR_URL")

wait_for_record "$ANALYZER_URL/api/engines?with_metadata=true" "$ENGINE_ID" "test-engine"
wait_for_record "$ANALYZER_URL/api/engines/$ENGINE_ID/query-groups" "$QUERY_GROUP_ID" "test-group"
wait_for_record "$ANALYZER_URL/api/engines/$ENGINE_ID/query_group/$QUERY_GROUP_ID/queries" "$QUERY_ID" "test-query"
