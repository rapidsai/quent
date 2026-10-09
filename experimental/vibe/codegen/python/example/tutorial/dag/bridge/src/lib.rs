// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused, clippy::all)]
mod instrumentation {
    include!(concat!(env!("OUT_DIR"), "/instrumentation.rs"));
}

include!(concat!(env!("OUT_DIR"), "/pyo3_bridge.rs"));
