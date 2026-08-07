// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runnable NVTX capture demo: debug-prints each captured event.
//!
//! ```text
//! pixi run cargo run -p nvtx-example
//! ```

use nvtx_bridge::NvtxEventEntity;
use quent_instrumentation::EventCallback;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The app's exporter: debug-print each captured NVTX event.
    let printer = EventCallback::<NvtxEventEntity>::new(|event| {
        println!("[{} @ {}] {:?}", event.id, event.timestamp, event.data.0);
    });

    nvtx_example::run_capture(Uuid::now_v7(), printer)
}
