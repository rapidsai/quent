<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Question CLI

Experimental Quent CLI for discovering query-engine captures and answering
deterministic questions from one query bundle.

This project has the unstable, agent-authored status described by the parent
[`vibe`](../README.md) directory.

## Setup

From the repository root:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli install
```

## Usage

List available commands:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask --help
```

Pass explicit IDs with `--json` for non-interactive automation:

```sh
pixi run pnpm --silent --dir experimental/vibe/quent-cli ask engines --json
pixi run pnpm --silent --dir experimental/vibe/quent-cli ask queries \
  --engine ENGINE_ID --json
```

Question commands use the same interactive query selector when engine or query
IDs are omitted:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask query-lifecycle
pixi run pnpm --dir experimental/vibe/quent-cli ask query-structure
pixi run pnpm --dir experimental/vibe/quent-cli ask operator-facts
pixi run pnpm --dir experimental/vibe/quent-cli ask operator-statistics-summary
```

`--json` changes the output format without disabling interactive selection.

## Validation

```sh
pixi run pnpm --dir experimental/vibe/quent-cli check
```
