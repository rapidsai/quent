// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use quent_analyzer::context::index_contexts;
use quent_instrumentation::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_query_engine_analyzer::ui::QuentViewer;
use quent_query_engine_server::analyzer_service_router;
use quent_simulator_analyzer::{SimulatorUiAnalyzer, Viewer};
use quent_simulator_fixed as fixed;
use quent_simulator_instrumentation as instrumentation;
use quent_simulator_store::Simulator;
use quent_store::event::{CombinedEventLoader, filesystem::Loader};
use serde_json::{Value, json};
use tower::ServiceExt;

type SimulatorContext = instrumentation::Context<instrumentation::Simulator>;

fn recorded_router() -> (tempfile::TempDir, axum::Router) {
    let output = tempfile::tempdir().unwrap();
    let context = SimulatorContext::try_new(ExporterOptions::FileSystem(
        FileSystemExporterOptions::new(FileSystemFormat::Ndjson, output.path().to_path_buf()),
    ))
    .unwrap();
    fixed::emit(&context);
    drop(context);
    let import_root = output.path().to_owned();
    let list_root = output.path().to_owned();
    let router = analyzer_service_router::<SimulatorUiAnalyzer>(
        Box::new(move |context_id| {
            let events = Loader::<Simulator>::new(
                &import_root,
                quent_store::context::ContextSet::one(context_id),
            )
            .combined_events()
            .map_err(quent_io::ImporterError::other)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(quent_io::ImporterError::other)?;
            Ok(Box::new(events.into_iter()))
        }),
        Box::new(move || index_contexts(&list_root, |dir| Ok(Viewer::context_inventory(dir)?))),
        None,
    )
    .unwrap();
    (output, router)
}

async fn logs(router: axum::Router, request: Value) -> (StatusCode, Vec<u8>) {
    let response = router
        .oneshot(
            Request::post(format!("/api/engines/{}/logs", fixed::ENGINE))
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, bytes)
}

#[tokio::test(flavor = "multi_thread")]
async fn exported_logs_are_filterable_through_http() {
    let (_output, router) = recorded_router();
    let (status, body) = logs(router.clone(), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let all: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(all["total"], 6);
    assert_eq!(all["items"][0]["timestamp_s"], 0.5);
    assert_eq!(all["sink_types"][0]["levels"].as_array().unwrap().len(), 5);
    let (status, body) = logs(
        router.clone(),
        json!({
            "window": {"start": 2.0, "end": 5.0}, "min_rank": 3,
            "level_names": ["warn", "error"], "sink_id": fixed::APP_LOG.to_string(),
            "sink_type_name": "AppLog", "message_regex": "(?i)spilling|error E[0-9]+",
            "page": {"max": 1, "page": 1}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let filtered: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(filtered["total"], 2);
    assert_eq!(filtered["items"].as_array().unwrap().len(), 1);
    let item = &filtered["items"][0];
    assert_eq!(item["level"]["name"], "error");
    assert_eq!(item["message"], "Example recoverable error E42");
    let worker = item["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|attr| attr["key"] == "worker_id")
        .unwrap();
    assert_eq!(worker["value"]["Struct"][0]["key"], "target");
    assert_eq!(
        worker["value"]["Struct"][0]["value"]["String"],
        fixed::WORKER_1.to_string()
    );
    let (status, body) = logs(router, json!({"message_regex": "café$"})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap()["total"], 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_log_filters_return_bad_request() {
    let (_output, router) = recorded_router();
    for request in [
        json!({"message_regex": "["}),
        json!({"window": {"start": 3.0, "end": 1.0}}),
        json!({"page": {"max": 0, "page": 0}}),
    ] {
        let (status, body) = logs(router.clone(), request).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{}",
            String::from_utf8_lossy(&body)
        );
    }
}
