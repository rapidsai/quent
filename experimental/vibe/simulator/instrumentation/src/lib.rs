// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Schema-generated simulator instrumentation.

#[cfg(feature = "collector")]
mod collector_sink;
pub mod nvtx;

#[cfg(feature = "collector")]
pub use collector_sink::SimulatorCollectorSink;
pub use nvtx::{
    DEFAULT_MAX_NVTX_RANGES, NvtxCapture, NvtxCategory, NvtxLayout, NvtxPushGuard, NvtxPushStack,
    NvtxResourceGuard, NvtxStartGuard,
};

include!(concat!(env!("OUT_DIR"), "/simulator.rs"));
