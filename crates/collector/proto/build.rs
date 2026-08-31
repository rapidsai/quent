// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=../../../proto");
    let file_descriptor_set = protox::compile(
        ["../../../proto/quent/collector/v1alpha/collector.proto"],
        ["../../../proto"],
    )?;
    tonic_prost_build::compile_fds(file_descriptor_set)?;
    Ok(())
}
