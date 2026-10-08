<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Schema Explorer

Experimental application for exploring Quent entity graphs, resource
timelines, state machines, records, and YAML models.

From the repository root:

```sh
pixi run --frozen pnpm --dir experimental/vibe/ui install --frozen-lockfile
pixi run --frozen pnpm --dir experimental/vibe/ui build
pixi run --frozen pnpm --dir experimental/vibe/ui explorer
```

Open the local URL printed by Vite, normally `http://localhost:5173`.

## Example links

Open a built-in model directly with the `example` query parameter, for example:

```text
https://rapidsai.github.io/quent/schema/?example=simple
```

Available values are `simple`, `hello`, `dynamo-inference`, `simulator`, and
`sirius`. Selecting another example updates the current URL without reloading
the page.

## YAML WebAssembly

The editor parses YAML through a browser build of `quent-yaml`. Regenerate the
local bindings after changing that crate:

```sh
pixi run --frozen pnpm --dir experimental/vibe/ui wasm:build
pixi run --frozen pnpm --dir experimental/vibe/ui --filter @quent-experimental/schema-explorer wasm:test
```

The generated bindings are ignored by Git. The explorer's dev, check, test,
and build commands regenerate them from the current checkout using Pixi's
Rust and `wasm-bindgen` tools.

The browser export is implemented by the sibling `yaml-wasm` crate, which
depends on `quent-yaml` without changing that crate.

## Browser tests

After building the workspace, run the production editor smoke test:

```sh
pixi run --frozen pnpm --dir experimental/vibe/ui --filter @quent-experimental/schema-explorer exec playwright install chromium
pixi run --frozen pnpm --dir experimental/vibe/ui --filter @quent-experimental/schema-explorer test:e2e
```

The Pages workflow runs this test with `SCHEMA_EXPLORER_BASE=/quent/schema/`
for both the build and test. It checks editor rendering, typing, and example
switching, and fails on browser exceptions or console errors.
