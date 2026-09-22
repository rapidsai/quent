// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

include!(concat!(env!("OUT_DIR"), "/pyo3_bridge.rs"));

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
#[unsafe(no_mangle)]
pub extern "C" fn quent_codegen_nvtx_test_emit() {
    nvtx::mark(c"python-generated-mark");
    let _range = nvtx::LocalRange::new(c"python-generated-range");
}

#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
#[unsafe(no_mangle)]
pub extern "C" fn quent_codegen_nvtx_test_emit() {}
