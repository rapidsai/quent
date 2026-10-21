// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#![cfg(not(target_arch = "wasm32"))]

use std::{
    io::Write,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use quent_analyzer::context::index_contexts;
use quent_events::{EntityRef, Event, EventPayload};
use quent_nvtx_sink::quent::nvtx::{self, NvtxEventAttributes, NvtxMessage, SinkEvent};
use quent_query_engine_analyzer::ui::QuentViewer;
use quent_query_engine_server::{
    analyzer_cache::AnalyzerCache, analyzer_service_router_with_cache,
};
use quent_simulator_analyzer::{SimulatorUiAnalyzer, Viewer};
use quent_simulator_store::{self as schema, EngineEvent};
use serde::Serialize;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const START: u64 = 2_000_000_000;

fn write_events<P: EventPayload + Serialize>(root: &Path, context: Uuid, events: &[Event<P>]) {
    let dir = root.join(context.to_string()).join(P::NAME);
    std::fs::create_dir_all(&dir).unwrap();
    let mut file = std::fs::File::create(dir.join("events.ndjson")).unwrap();
    for event in events {
        serde_json::to_writer(&mut file, event).unwrap();
        writeln!(file).unwrap();
    }
}

async fn get(app: &Router, uri: String) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn schema_loader_and_cached_application_analysis_serve_independent_nvtx_sinks() {
    let root = tempfile::tempdir().unwrap();
    let context = Uuid::from_u128(1);
    let engine = Uuid::from_u128(2);
    let process = Uuid::from_u128(3);
    let sinks = [Uuid::from_u128(4), Uuid::from_u128(5)];
    write_events(
        root.path(),
        context,
        &[
            Event::new(
                engine,
                START,
                EngineEvent::Init {
                    implementation: schema::EngineImplementationAttributes {
                        name: None,
                        version: None,
                        custom_attributes: Default::default(),
                    },
                    instance_name: None,
                    configuration: schema::EngineConfiguration {
                        workers: 0,
                        threads_per_worker: 0,
                        gpus_per_worker: 0,
                        num_workloads: 0,
                        num_queries: 0,
                    },
                },
            ),
            Event::new(engine, START + 3_000_000_000, EngineEvent::Exit),
        ],
    );
    write_events(
        root.path(),
        context,
        &[Event::new(
            process,
            START,
            schema::ProcessEvent::Started {
                identity: schema::quent::os::Process { native_id: 42 },
            },
        )],
    );
    let mut events = Vec::new();
    for (index, sink) in sinks.into_iter().enumerate() {
        events.extend([
            Event::new(
                sink,
                START,
                SinkEvent::Initialized {
                    process: EntityRef::new(process, ()),
                },
            ),
            Event::new(
                sink,
                START,
                SinkEvent::RangeStart {
                    data: nvtx::RangeStart {
                        domain: 0,
                        range_id: 1,
                        attributes: NvtxEventAttributes {
                            message: Some(NvtxMessage {
                                string: Some(format!("sink {index}")),
                                registered_handle: None,
                            }),
                            ..Default::default()
                        },
                    },
                },
            ),
            Event::new(
                sink,
                START + (index as u64 + 1) * 1_000_000_000,
                SinkEvent::RangeEnd {
                    data: nvtx::RangeEnd {
                        domain: 0,
                        range_id: 1,
                    },
                },
            ),
        ]);
    }
    write_events(root.path(), context, &events);

    quent_events::build_info::ArtifactInfo::new(quent_events::build_info::model_info::<
        schema::Simulator,
    >("Simulator"))
    .write_sidecar(&root.path().join(context.to_string()))
    .unwrap();

    let loads = Arc::new(AtomicUsize::new(0));
    let importer_loads = Arc::clone(&loads);
    let importer_root = root.path().to_owned();
    let lister_root = root.path().to_owned();
    let analyzers = AnalyzerCache::<SimulatorUiAnalyzer>::new(
        Box::new(move |id| {
            importer_loads.fetch_add(1, Ordering::SeqCst);
            Ok(Viewer::import_events(&importer_root.join(id.to_string()))?)
        }),
        Box::new(move || index_contexts(&lister_root, |dir| Ok(Viewer::context_inventory(dir)?))),
    );
    let app = analyzer_service_router_with_cache(
        analyzers.clone(),
        None,
        Router::new().nest("/api/engines", super::routes(analyzers)),
    )
    .unwrap();
    let base = format!("/api/engines/{engine}");
    assert_eq!(
        get(&app, format!("{base}/nvtx")).await,
        (StatusCode::OK, json!(sinks))
    );
    for (index, sink) in sinks.into_iter().enumerate() {
        let uri = format!("{base}/nvtx/{sink}");
        let (status, catalog) = get(&app, format!("{uri}/catalog?query_start={START}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(catalog["trace_end"], (index + 1) as f64);
        let response = app.clone().oneshot(Request::post(format!("{uri}/viewport?query_start={START}"))
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "viewport": { "start": 0.0, "end": 3.0 },
                "selections": [{ "domain_id": "0", "category_ids": [], "include_uncategorized": true }],
            }).to_string())).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let viewport: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(
            viewport["statistics"][0]["message"],
            format!("sink {index}")
        );
        assert_eq!(viewport["statistics"].as_array().unwrap().len(), 1);
        assert_eq!(
            viewport["statistics"][0]["total_duration"],
            (index + 1) as f64
        );
    }
    assert_eq!(get(&app, base).await.0, StatusCode::OK);
    assert_eq!(
        loads.load(Ordering::SeqCst),
        1,
        "all views share the application analyzer cache"
    );
    let response = app
        .oneshot(
            Request::get(format!(
                "/api/engines/{engine}/nvtx/{engine}/catalog?query_start={START}"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
