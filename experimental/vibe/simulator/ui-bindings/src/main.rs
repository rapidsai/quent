// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../ui/generated/ts-bindings");
    quent_simulator_ui_bindings::generate(&output_dir)
}
