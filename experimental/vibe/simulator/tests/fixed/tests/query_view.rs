// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_instrumentation::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_query_engine_analyzer::{
    QueryEngineModel, QueryEntity,
    ui::{ContextEvent, QuentViewer, UiAnalyzer},
};
use quent_simulator::{SimulationConfig, simulate};
use quent_simulator_analyzer::{SimulatorUiAnalyzer, Viewer};
use quent_simulator_instrumentation as instrumentation;
use quent_simulator_store::Simulator;
use quent_store::event::filesystem::Store;

type SimulatorContext = instrumentation::Context<instrumentation::Simulator>;

static SIMULATION_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn assert_nvtx_routes_share_the_application_analyzer(
    root: &std::path::Path,
    context_id: uuid::Uuid,
    engine_id: uuid::Uuid,
) {
    use std::collections::BTreeSet;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use axum::{body::Body, http::Request};
    use quent_analyzer::context::{ContextIndex, ContextInventory};
    use quent_query_engine_server::{error::ServerError, model_viewer_router};
    use tower::ServiceExt;

    let imports = Arc::new(AtomicUsize::new(0));
    let importer_root = root.to_path_buf();
    let importer_calls = Arc::clone(&imports);
    let importer = move |requested_context_id: uuid::Uuid| {
        importer_calls.fetch_add(1, Ordering::SeqCst);
        Ok::<_, ServerError>(Viewer::import_events(
            &importer_root.join(requested_context_id.to_string()),
        )?)
    };
    let lister = move || {
        let mut index = ContextIndex::default();
        index.add_inventory(
            context_id.into(),
            ContextInventory {
                analysis_target_ids: BTreeSet::from([engine_id]),
            },
        );
        Ok::<_, ServerError>(index)
    };
    let app = model_viewer_router::<Viewer>(Box::new(importer), Box::new(lister), None).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        for _ in 0..2 {
            let response = app
                .clone()
                .oneshot(
                    Request::get(format!(
                        "/api/nvtx/contexts/{context_id}/catalog?query_start=0"
                    ))
                    .body(Body::empty())
                    .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), axum::http::StatusCode::OK);
        }
    });
    assert_eq!(imports.load(Ordering::SeqCst), 1);
}

#[test]
fn analyzer_routes_distinguish_present_empty_and_missing_nvtx_streams() {
    let _simulation_guard = SIMULATION_TEST_LOCK.lock().unwrap();

    use std::collections::BTreeSet;

    use axum::{body::Body, http::Request};
    use quent_analyzer::context::{ContextIndex, ContextInventory};
    use quent_events::EntityEvent;
    use quent_query_engine_server::{error::ServerError, model_viewer_router};
    use tower::ServiceExt;

    fn create_context(root: &std::path::Path) -> (uuid::Uuid, uuid::Uuid) {
        let context = SimulatorContext::try_new_with_options(
            ExporterOptions::FileSystem(FileSystemExporterOptions::new(
                FileSystemFormat::Ndjson,
                root.to_path_buf(),
            )),
            instrumentation::ContextOptions::default()
                .with_source_capture(instrumentation::SourceCapture::Disabled),
        )
        .unwrap();
        let context_id = context.id();
        simulate(
            context,
            SimulationConfig {
                num_queries: 1,
                num_tasks: 1,
                num_workers: 1,
                num_threads: 1,
                num_gpus: 1,
                ..SimulationConfig::default()
            },
        );
        let inventory = Viewer::context_inventory(&root.join(context_id.to_string())).unwrap();
        let engine_id = inventory.analysis_target_ids.into_iter().next().unwrap();
        (context_id, engine_id)
    }

    fn router(
        root: &std::path::Path,
        context_id: uuid::Uuid,
        engine_id: uuid::Uuid,
    ) -> axum::Router {
        let importer_root = root.to_path_buf();
        let importer = move |requested_context_id: uuid::Uuid| {
            Ok::<_, ServerError>(Viewer::import_events(
                &importer_root.join(requested_context_id.to_string()),
            )?)
        };
        let lister = move || {
            let mut index = ContextIndex::default();
            index.add_inventory(
                context_id.into(),
                ContextInventory {
                    analysis_target_ids: BTreeSet::from([engine_id]),
                },
            );
            Ok::<_, ServerError>(index)
        };
        model_viewer_router::<Viewer>(Box::new(importer), Box::new(lister), None).unwrap()
    }

    let empty_root = tempfile::tempdir().unwrap();
    let (empty_context_id, empty_engine_id) = create_context(empty_root.path());
    std::fs::create_dir_all(
        empty_root
            .path()
            .join(empty_context_id.to_string())
            .join(<quent_simulator_store::NvtxEventEvent as EntityEvent>::NAME),
    )
    .unwrap();
    let empty_app = router(empty_root.path(), empty_context_id, empty_engine_id);

    let missing_root = tempfile::tempdir().unwrap();
    let (missing_context_id, missing_engine_id) = create_context(missing_root.path());
    std::fs::remove_dir_all(
        missing_root
            .path()
            .join(missing_context_id.to_string())
            .join(<quent_simulator_store::NvtxEventEvent as EntityEvent>::NAME),
    )
    .unwrap();
    let missing_app = router(missing_root.path(), missing_context_id, missing_engine_id);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let empty = empty_app
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/{empty_context_id}/catalog?query_start=0"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(empty.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(empty.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = std::str::from_utf8(&body).unwrap();
        assert!(
            body.contains("\"domains\":[]"),
            "catalog was not empty: {body}"
        );

        let missing = missing_app
            .oneshot(
                Request::get(format!(
                    "/api/nvtx/contexts/{missing_context_id}/catalog?query_start=0"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), axum::http::StatusCode::NOT_FOUND);
    });
}

#[test]
fn builds_each_query_view_when_queries_share_resources() {
    let _simulation_guard = SIMULATION_TEST_LOCK.lock().unwrap();

    let output = tempfile::tempdir().unwrap();
    let context = SimulatorContext::try_new(ExporterOptions::FileSystem(
        FileSystemExporterOptions::new(FileSystemFormat::Ndjson, output.path().to_path_buf()),
    ))
    .unwrap();
    let context_id = context.id();
    simulate(
        context,
        SimulationConfig {
            num_queries: 2,
            num_tasks: 1,
            num_workers: 1,
            num_threads: 1,
            num_gpus: 1,
            ..SimulationConfig::default()
        },
    );

    let context_dir = output.path().join(context_id.to_string());
    let inventory = Viewer::context_inventory(&context_dir).unwrap();
    let engine_id = inventory.analysis_target_ids.into_iter().next().unwrap();
    let events = Store::<Simulator>::new(output.path())
        .load_context(context_id)
        .unwrap()
        .into_events();
    let events = events
        .into_iter()
        .map(|event| ContextEvent::new(context_id.into(), event));
    let analyzer = SimulatorUiAnalyzer::try_new_from_contexts(engine_id, events).unwrap();

    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    {
        let sources = analyzer.model.nvtx_sources();
        assert_eq!(sources.len(), 1);
        let source = &sources[0];
        assert_eq!(source.context_id(), context_id);
        assert!(!source.model().spans().is_empty());
        assert!(source.model().spans().iter().all(|span| {
            analyzer
                .model
                .task_executor_thread_for_nvtx_span(source, span)
                .is_some()
        }));

        assert_nvtx_routes_share_the_application_analyzer(output.path(), context_id, engine_id);
    }

    let query_ids = analyzer
        .query_engine_model()
        .queries()
        .map(|query| query.to_ui().unwrap().id)
        .collect::<Vec<_>>();

    assert_eq!(query_ids.len(), 2);
    for query_id in query_ids {
        assert_eq!(analyzer.query_bundle(query_id).unwrap().query_id, query_id);
    }
}
