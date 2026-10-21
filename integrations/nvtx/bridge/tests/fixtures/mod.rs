// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::Event;
use quent_instrumentation::{Context, EventCallback};
use quent_nvtx_bridge::Capture;
use quent_nvtx_sink::quent::nvtx::SinkEvent;

#[allow(unused)]
pub mod application {
    include!(concat!(env!("OUT_DIR"), "/nvtx_capture.rs"));
}

pub type ApplicationContext = Context<application::NvtxCapture>;
pub use application::quent::nvtx::Sink;

pub fn context(callback: impl Fn(Event<SinkEvent>) + Send + Sync + 'static) -> ApplicationContext {
    ApplicationContext::try_new(EventCallback::<application::NvtxCaptureEvent>::new(
        move |event| {
            if let application::NvtxCaptureEvent::Quent(application::quent::QuentEvent::Nvtx(
                application::quent::nvtx::NvtxEvent::Sink(data),
            )) = event.data
            {
                callback(Event::new(event.id, event.timestamp, data));
            }
        },
    ))
    .unwrap()
}

#[allow(dead_code)]
pub fn start(
    callback: impl Fn(Event<SinkEvent>) + Send + Sync + 'static,
) -> (ApplicationContext, Capture) {
    let context = context(move |event| {
        if !matches!(event.data, SinkEvent::Initialized { .. }) {
            callback(event);
        }
    });
    let mut process = context.observer::<application::Process>().handle();
    process
        .started(application::quent::os::Process {
            native_id: std::process::id(),
        })
        .unwrap();
    let capture =
        Capture::install(context.observer::<Sink>(), process.as_any_entity_ref()).unwrap();
    (context, capture)
}
