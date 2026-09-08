<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Experimental Schema Explorer workspace

This is a self-contained pnpm and Cargo workspace. It does not register
packages, scripts, catalogs, or dependencies in the repository's main `ui`
workspace and does not add features to existing Rust crates.

The workspace consumes existing code through dependencies:

- `schema-viewer` links to the generated `@quent/schema` TypeScript package.
- `yaml-wasm` depends on the existing `quent-yaml` and `quent-schema` crates.
- `schema-explorer` depends on the local `schema-viewer` package.

From the repository root:

```sh
pixi run --frozen pnpm --dir experimental/vibe/ui install --frozen-lockfile
pixi run --frozen pnpm --dir experimental/vibe/ui schema:ci
pixi run --frozen pnpm --dir experimental/vibe/ui explorer
```

The explorer is normally available at `http://localhost:5173`.
