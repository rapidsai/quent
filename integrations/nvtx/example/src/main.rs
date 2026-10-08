// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Runnable NVTX capture demo: debug-prints each captured event.
//!
//! ```text
//! pixi run cargo run -p quent-nvtx-example
//! ```

use quent_instrumentation::{ContextInner, EventCallback};
use quent_nvtx_bridge::Capture;
use quent_nvtx_events::NvtxEvent;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let session = Uuid::now_v7();
    let printer = EventCallback::<NvtxEvent>::new(|event| {
        println!("[{} @ {}] {:?}", event.id, event.timestamp, event.data);
    });
    // TODO(johanpel): Move NVTX events into the schema so this example can use a generated
    // Context + Observer instead of ContextInner.
    let context = ContextInner::try_new(session)?;
    let observer = context.block_on(async { context.observer::<NvtxEvent>(&printer).await })?;

    let capture = Capture::install(session, observer)?;

    nvtx::mark(c"startup");
    let local_range = nvtx::LocalRange::new(c"phase-1");
    drop(local_range);
    let range = nvtx::Range::new(c"phase-2");
    drop(range);

    drop(capture);
    Ok(())
}
