#!/usr/bin/env sh
# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

set -eu

rustc="$(rustup which --toolchain 1.97.0 rustc)"
cargo="$(rustup which --toolchain 1.97.0 cargo)"

RUSTC="$rustc" "$cargo" build \
  --manifest-path ../yaml-wasm/Cargo.toml \
  --target wasm32-unknown-unknown \
  --release
wasm-bindgen \
  ../yaml-wasm/target/wasm32-unknown-unknown/release/quent_schema_explorer_yaml.wasm \
  --target web \
  --out-dir wasm \
  --out-name quent_yaml
