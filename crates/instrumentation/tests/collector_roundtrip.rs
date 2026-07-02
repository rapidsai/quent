// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A client whose exporter is the collector: events must survive the full
//! gRPC round-trip, and dropping the context (which flushes the collector
//! client) must not panic — exercising the teardown path the bench masks with
//! `mem::forget`.

#![cfg(feature = "collector")]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::{TestEvent, TestModel};
use quent_build_info::ModelSource;
use quent_collector::{CollectorSink, server::CollectorService};
use quent_collector_proto::collector_server::CollectorServer;
use quent_events::{EntityEvent, Event};
use quent_exporter::{CollectorExporterOptions, ExporterOptions};
use quent_instrumentation::Context;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server as GrpcServer;
use uuid::Uuid;

const EVENTS: usize = 100;

/// Collector-side sink: decodes each event and counts it.
struct CountingSink {
    received: Arc<AtomicUsize>,
}

impl CollectorSink for CountingSink {
    fn ingest(&self, entity: &str, event: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if entity != TestEvent::NAME {
            return Err(format!("unexpected entity `{entity}`").into());
        }
        let _: Event<TestEvent> = ciborium::from_reader(event)?;
        self.received.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Start an in-process collector server on a random localhost port. Returns the
/// dial address and the server's runtime — the caller keeps the runtime alive
/// for the test and drops it to shut the server down.
fn start_server(received: Arc<AtomicUsize>) -> (http::Uri, tokio::runtime::Runtime) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address: http::Uri = format!("http://{}", std_listener.local_addr().unwrap())
        .parse()
        .unwrap();
    std_listener.set_nonblocking(true).unwrap();

    rt.spawn(async move {
        let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
        let service = CollectorService::new(move |_id| {
            Ok::<_, String>(CountingSink {
                received: received.clone(),
            })
        });
        let _ = GrpcServer::builder()
            .add_service(CollectorServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await;
    });

    (address, rt)
}

#[test]
fn collector_client_flushes_all_events_on_drop() {
    let received = Arc::new(AtomicUsize::new(0));
    // `_server` keeps the collector running for the test and shuts it down on
    // drop at the end.
    let (address, _server) = start_server(received.clone());

    // A plain sync client (no ambient runtime); the context spawns its own.
    let ctx = Context::try_new(
        TestModel::model_info(),
        Some(ExporterOptions::Collector(CollectorExporterOptions {
            address,
        })),
    )
    .unwrap();
    {
        let observer = ctx.block_on(ctx.observer::<TestEvent>()).unwrap();
        for _ in 0..EVENTS {
            observer.emit(Uuid::now_v7(), TestEvent);
        }
        // Dropping the observer shuts down the collector client and tears down
        // its tasks — this is what must not panic.
    }
    drop(ctx);

    // Delivery completes asynchronously on the server; poll until all arrive.
    let deadline = Instant::now() + Duration::from_secs(10);
    while received.load(Ordering::SeqCst) < EVENTS && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        received.load(Ordering::SeqCst),
        EVENTS,
        "collector did not receive all events"
    );
}
