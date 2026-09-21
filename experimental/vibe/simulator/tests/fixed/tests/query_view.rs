// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::Event;
use quent_instrumentation::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_query_engine_analyzer::{
    QueryEngineModel, QueryEntity,
    ui::{QuentViewer, UiAnalyzer},
};
use quent_simulator::{SimulationConfig, simulate};
use quent_simulator_analyzer::{SimulatorUiAnalyzer, Viewer};
use quent_simulator_instrumentation as instrumentation;
use quent_simulator_store::{Simulator, SimulatorEvent};
use quent_store::event::{ModelEventStore, filesystem::Store};

type SimulatorContext = instrumentation::Context<instrumentation::Simulator>;

#[test]
fn builds_each_query_view_when_queries_share_resources() {
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
        .events(context_id)
        .unwrap()
        .collect::<Result<Vec<Event<SimulatorEvent>>, _>>()
        .unwrap();
    let analyzer = SimulatorUiAnalyzer::try_new(engine_id, events.into_iter()).unwrap();
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
