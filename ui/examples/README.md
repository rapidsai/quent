# UI Plugin POCs

This directory contains proof-of-concept plugin integrations that consume the modularized `@quent/*` packages in isolation.

## Packages

- `grafana-dag-plugin`: Grafana-style panel plugin shell rendering `DAGChart`.
- `superset-timeline-plugin`: Superset-style chart plugin shell rendering `Timeline` + `TimelineController`.

## Run an example

From `ui/`:

- `pnpm --filter grafana-dag-plugin dev`
- `pnpm --filter superset-timeline-plugin dev`
