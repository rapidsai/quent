// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Captures the quent repository's git provenance at build time and exposes it
//! to the crate as `QUENT_BUILD_*` env vars (read by `quent_build_info::quent`).

use std::path::PathBuf;

// Shared git capture (see `src/git.rs`). Included rather than imported because a
// crate's own `build.rs` cannot depend on its library.
include!("src/git.rs");

fn main() {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    emit("QUENT_BUILD", &manifest_dir);
}
