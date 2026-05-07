// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for PyO3 bridge code generation.

use quent_codegen::{PyO3Options, emit_pyo3, emit_pyo3_stubs};
use quent_model::{EntityDef, EntityEventDef, ModelBuilder};

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
    assert!(!file.content.contains("PyCustomAttributes"));
    assert!(file.content.contains("pub struct PyWorkerObserver"));
    assert!(file.content.contains("pub struct PyFileStatsHandle"));
    assert!(file.content.contains("pub struct PyTaskHandle"));
    assert!(file.content.contains("pub fn worker"));
    assert!(!file.content.contains("pub fn worker_declaration"));
    assert!(
        file.content
            .contains("pub fn create(&self, id: PyRef<'_, PyUuid>)")
    );
    assert!(file.content.contains("parent_group_id: PyRef<'_, PyUuid>"));
    assert!(file.content.contains("pub fn checksum("));
    assert!(!file.content.contains("pub fn checksum(&self, id:"));
    assert!(file.content.contains("pub fn queued"));
    assert!(!file.content.contains("self.inner.queued("));
    assert!(file.content.contains("__usage_arg_item"));
    assert!(file.content.contains("expected dict for custom attributes"));
    assert!(file.content.contains("value.cast::<PyBool>()"));
    assert!(
        file.content
            .contains("extract::<PyRef<'_, PyQueueHandle>>()")
    );
    assert!(
        file.content
            .contains("extract::<PyRef<'_, PyThreadHandle>>()")
    );
    assert!(!file.content.contains("expected Uuid or Quent handle"));
    assert!(!file.content.contains("__extract_uuid(&resource_obj)"));
    assert!(
        file.content
            .contains("#[pymodule(name = \"quent_readme\")]")
    );
}

#[test]
fn generate_readme_pyo3_type_stubs() {
    let builder = quent_readme_example::AppModel::build("App");
    let options = PyO3Options {
        module_name: "quent_readme".into(),
        instrumentation_crate: "quent_readme_example".into(),
    };

    let files = emit_pyo3_stubs(&builder, &options);
    assert_eq!(files.len(), 2);

    let file = &files[0];
    assert_eq!(file.name, "quent_readme/__init__.pyi");
    assert!(file.content.contains("class Uuid:"));
    assert!(file.content.contains("def now_v7() -> Uuid"));
    assert!(file.content.contains("class Context:"));
    assert!(file.content.contains("class DetailsDict(TypedDict):"));
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
        !file
            .content
            .contains("def queued(self, instance_name: str, index: int")
    );
    assert!(file.content.contains("thread: ThreadHandle | None"));
    assert!(
        file.content
            .contains("QueueHandle | tuple[QueueHandle, int] | None")
    );
    assert!(
        file.content
            .contains("custom: Mapping[str, bool | int | float | str | None]")
    );
    assert!(!file.content.contains("Mapping[str, object]"));
    assert!(!file.content.contains("Uuid | QueueHandle"));

    let marker = &files[1];
    assert_eq!(marker.name, "quent_readme/py.typed");
    assert!(marker.content.is_empty());
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
    assert!(
        bridge
            .content
            .contains("worker_id: Option<PyRef<'_, PyUuid>>")
    );
    assert!(!bridge.content.contains("self.inner.init("));

    let files = emit_pyo3_stubs(&builder, &options);
    assert_eq!(files.len(), 2);
    let stubs = &files[0];
    assert_eq!(stubs.name, "quent_qe/__init__.pyi");
    assert!(
        stubs
            .content
            .contains("class EngineImplementationAttributesDict")
    );
    assert!(
        stubs
            .content
            .contains("custom_attributes: Mapping[str, bool | int | float | str | None]")
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
    assert!(stubs.content.contains(
        "def init(self, id: Uuid, instance_name: str, query_group_id: Uuid) -> QueryHandle"
    ));
    assert!(
        !stubs
            .content
            .contains("def init(self, instance_name: str, query_group_id: Uuid) -> None")
    );
    let marker = &files[1];
    assert_eq!(marker.name, "quent_qe/py.typed");
    assert!(marker.content.is_empty());
}

#[test]
fn dotted_pyo3_module_name_uses_export_basename() {
    let builder = quent_readme_example::AppModel::build("App");
    let options = PyO3Options {
        module_name: "quent_pkg._native".into(),
        instrumentation_crate: "quent_readme_example".into(),
    };

    let bridge = emit_pyo3(&builder, &options).remove(0);
    syn::parse_file(&bridge.content).unwrap_or_else(|e| panic!("{}: {}", bridge.name, e));
    assert!(bridge.content.contains("#[pymodule(name = \"_native\")]"));
    assert!(bridge.content.contains("pub fn quent_pkg__native"));

    let stubs = emit_pyo3_stubs(&builder, &options);
    assert_eq!(stubs.len(), 2);
    assert_eq!(stubs[0].name, "quent_pkg/_native.pyi");
    assert_eq!(stubs[1].name, "quent_pkg/py.typed");
}

fn keyword_named_model() -> ModelBuilder {
    let mut builder = ModelBuilder::new("Keyword");
    builder.add_entity(EntityDef {
        name: "class".into(),
        events: vec![EntityEventDef {
            name: "class_declaration".into(),
            attributes: Vec::new(),
        }],
        module_path: "crate::keyword".into(),
    });
    builder
}

#[test]
fn pyo3_bridge_and_stubs_share_python_keyword_escaping() {
    let builder = keyword_named_model();
    let options = PyO3Options {
        module_name: "keyword".into(),
        instrumentation_crate: "crate".into(),
    };

    let bridge = emit_pyo3(&builder, &options).remove(0);
    syn::parse_file(&bridge.content).unwrap_or_else(|e| panic!("{}: {}", bridge.name, e));
    assert!(bridge.content.contains("pub fn class_observer"));
    assert!(bridge.content.contains("pub fn class_(&self"));

    let stubs = emit_pyo3_stubs(&builder, &options).remove(0);
    assert!(
        stubs
            .content
            .contains("def class_observer(self) -> ClassObserver")
    );
    assert!(stubs.content.contains("def class_(self, id: Uuid) -> Uuid"));
}
