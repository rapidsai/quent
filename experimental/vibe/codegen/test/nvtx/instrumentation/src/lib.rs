// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared generated instrumentation for the C++ and Python live-capture tests.

#[allow(unused, clippy::all)]
pub mod model {
    include!(concat!(env!("OUT_DIR"), "/instrumentation.rs"));
}
