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

## YAML WebAssembly

The editor parses YAML through a browser build of `quent-yaml`. Regenerate the
checked-in bindings after changing that crate:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.97.0
cargo install wasm-bindgen-cli --version 0.2.126 --locked
pixi run --frozen pnpm --dir experimental/vibe/ui wasm:build
pixi run --frozen pnpm --dir experimental/vibe/ui --filter @quent-experimental/schema-explorer wasm:test
```

The browser export is implemented by the sibling `yaml-wasm` crate, which
depends on `quent-yaml` without changing that crate.
