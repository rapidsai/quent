// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use sha2::{Digest, Sha256};

fn main() {
    const SCHEMA: &[u8] = include_bytes!("../model.yaml");

    let digest = Sha256::digest(SCHEMA)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    println!("cargo:rerun-if-changed=../model.yaml");
    println!("cargo:rustc-env=QUENT_SIMULATOR_SCHEMA_DIGEST=sha256:{digest}");
}
