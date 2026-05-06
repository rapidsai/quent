// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for PyO3 bridge code generation.

use quent_codegen::{PyO3Options, emit_pyo3, emit_pyo3_stubs};

#[test]
fn generate_readme_pyo3_bridge() {
    let builder = quent_readme_example::AppModel::build("App");
    let options = PyO3Options {
        module_name: "quent_readme".into(),
        instrumentation_crate: "quent_readme_example".into(),
    };

    let files = emit_pyo3(&builder, &options);
    assert_eq!(files.len(), 1);

    let file = &files[0];
    assert_eq!(file.name, "pyo3_bridge.rs");
    syn::parse_file(&file.content).unwrap_or_else(|e| panic!("{}: {}", file.name, e));

    assert!(file.content.contains("pub fn quent_readme"));
    assert!(file.content.contains("pub struct PyUuid"));
    assert!(file.content.contains("pub fn now_v7() -> PyUuid"));
    assert!(file.content.contains("pub struct PyContext"));
    assert!(file.content.contains("pub struct PyCustomAttributes"));
    assert!(file.content.contains("pub struct PyWorkerObserver"));
    assert!(file.content.contains("pub struct PyFileStatsHandle"));
    assert!(file.content.contains("pub struct PyTaskHandle"));
    assert!(file.content.contains("pub fn worker"));
    assert!(!file.content.contains("pub fn worker_declaration"));
    assert!(
        file.content
            .contains("pub fn create(&self, id: &Bound<'_, PyAny>)")
    );
    assert!(file.content.contains("pub fn checksum("));
    assert!(!file.content.contains("pub fn checksum(&self, id:"));
    assert!(file.content.contains("pub fn queued"));
    assert!(!file.content.contains("self.inner.queued("));
    assert!(file.content.contains("__usage_arg_item"));
    assert!(
        file.content
            .contains("extract::<PyRef<'_, PyQueueHandle>>()")
    );
    assert!(
        file.content
            .contains("extract::<PyRef<'_, PyThreadHandle>>()")
    );
    assert!(!file.content.contains("__extract_uuid(&resource_obj)"));
}

#[test]
fn generate_readme_pyo3_type_stubs() {
    let builder = quent_readme_example::AppModel::build("App");
    let options = PyO3Options {
        module_name: "quent_readme".into(),
        instrumentation_crate: "quent_readme_example".into(),
    };

    let files = emit_pyo3_stubs(&builder, &options);
    assert_eq!(files.len(), 1);

    let file = &files[0];
    assert_eq!(file.name, "quent_readme.pyi");
    assert!(file.content.contains("class Uuid:"));
    assert!(file.content.contains("def now_v7() -> Uuid"));
    assert!(file.content.contains("class Context:"));
    assert!(
        file.content
            .contains("class DetailsDict(typing.TypedDict):")
    );
    assert!(file.content.contains("def worker(self, id: Uuid"));
    assert!(
        file.content
            .contains("def cluster(self, id: Uuid, instance_name: str) -> Uuid")
    );
    assert!(
        file.content
            .contains("def create(self, id: Uuid) -> FileStatsHandle")
    );
    assert!(
        file.content
            .contains("def checksum(self, algorithm: str, value: str) -> None")
    );
    assert!(!file.content.contains("def worker_declaration"));
    assert!(
        file.content
            .contains("def queued(self, id: Uuid, instance_name: str, index: int")
    );
    assert!(
        file.content
            .contains("def queued(self, instance_name: str, index: int")
    );
    assert!(
        file.content
            .contains("thread: typing.Optional[ThreadHandle]")
    );
    assert!(
        file.content
            .contains("typing.Union[QueueHandle, typing.Tuple[QueueHandle, int]]")
    );
    assert!(!file.content.contains("typing.Union[Uuid, QueueHandle"));
}

#[test]
fn generate_query_engine_pyo3_bridge_and_stubs() {
    let builder = quent_query_engine_model::QueryEngineModel::build("QueryEngine");
    let options = PyO3Options {
        module_name: "quent_qe".into(),
        instrumentation_crate: "quent_qe_python_instrumentation".into(),
    };

    let files = emit_pyo3(&builder, &options);
    assert_eq!(files.len(), 1);
    let bridge = &files[0];
    syn::parse_file(&bridge.content).unwrap_or_else(|e| panic!("{}: {}", bridge.name, e));
    assert!(bridge.content.contains("pub fn quent_qe"));
    assert!(bridge.content.contains("pub struct PyEngineHandle"));
    assert!(bridge.content.contains("pub struct PyOperatorHandle"));
    assert!(bridge.content.contains("pub struct PyQueryHandle"));
    assert!(bridge.content.contains("self.inner.init("));

    let files = emit_pyo3_stubs(&builder, &options);
    assert_eq!(files.len(), 1);
    let stubs = &files[0];
    assert_eq!(stubs.name, "quent_qe.pyi");
    assert!(
        stubs
            .content
            .contains("class EngineImplementationAttributesDict")
    );
    assert!(stubs.content.contains("class PlanParentDict"));
    assert!(
        stubs
            .content
            .contains("def create(self, id: Uuid) -> EngineHandle")
    );
    assert!(
        stubs
            .content
            .contains("def declaration(self, plan_id: Uuid")
    );
    assert!(
        stubs
            .content
            .contains("def init(self, instance_name: str, query_group_id: Uuid) -> None")
    );
}
