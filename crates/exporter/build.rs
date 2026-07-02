// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Sets the `filesystem` cfg when any filesystem format feature is enabled, so
// the filesystem exporter/importer code can gate on it without exposing an
// internal marker feature.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(filesystem)");
    let enabled = ["NDJSON", "MSGPACK", "POSTCARD"]
        .iter()
        .any(|f| std::env::var_os(format!("CARGO_FEATURE_{f}")).is_some());
    if enabled {
        println!("cargo::rustc-cfg=filesystem");
    }
}
