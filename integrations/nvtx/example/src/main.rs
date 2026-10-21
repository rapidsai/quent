// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused)]
mod instrumentation {
    include!(concat!(env!("OUT_DIR"), "/nvtx_sink.rs"));
}

use instrumentation::{Context, NvtxSink, NvtxSinkEvent, Process, quent::nvtx::Sink};
use quent_instrumentation::EventCallback;
use quent_nvtx_bridge::Capture;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let printer = EventCallback::<NvtxSinkEvent>::new(|event| {
        println!("[{} @ {}] {:?}", event.id, event.timestamp, event.data);
    });
    let context = Context::<NvtxSink>::try_new(printer)?;
    let mut process = context.observer::<Process>().handle();
    process.started(instrumentation::quent::os::Process {
        native_id: std::process::id(),
    })?;
    let capture = Capture::install(context.observer::<Sink>(), process.as_any_entity_ref())?;

    let domain = nvtx::Domain::new(c"application");
    domain.mark(
        domain
            .event_attributes_builder()
            .message(c"started")
            .payload(1.5_f64)
            .build(),
    );
    let range = domain.range(c"work");
    drop(range);
    drop(domain);

    drop(capture);
    Ok(())
}
