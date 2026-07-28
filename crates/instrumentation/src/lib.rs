// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Backing structures for generated instrumentation libraries.
//!
//! Instrumented application code should not import this crate directly unless
//! there is a very special reason. Instead, it should interact with the
//! generated instrumentation library only.

mod context;
mod entity_ref;
mod handle;
mod observer;
mod sidecar;

pub use context::Context;
pub use entity_ref::{AnyEntity, EntityRef};
pub use handle::{Handle, HandleError};
pub use observer::{EventSender, Observer};
pub use sidecar::write_sidecar;

// Re-export everything the generated instrumentation code references, so a
// consumer needs only the `quent-instrumentation` dependency, selecting an
// exporter backend through its `io-*` features.
pub use quent_build_info as build_info;
pub use quent_dynamic_attributes::DynamicAttributes;
pub use quent_events::{EntityEvent, Event};
pub use quent_io::ExporterOptions;
pub use uuid::Uuid;

/// A caller-supplied event sink, selected via the `io-callback` feature.
#[cfg(feature = "io-callback")]
pub use quent_io::EventCallback;

#[cfg(test)]
mod tests {
    use super::*;
    use quent_build_info::ModelSource;
    use quent_events::{EntityEvent, Event};
    use quent_io::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
    use uuid::Uuid;

    struct TestModel;

    impl ModelSource for TestModel {
        fn package() -> &'static str {
            "quent-instrumentation"
        }
        fn source() -> quent_build_info::BuildInfo {
            quent_build_info::BuildInfo::unknown()
        }
    }

    #[derive(Debug, serde::Serialize)]
    struct TestEvent;

    impl EntityEvent for TestEvent {
        const NAME: &'static str = "TestEvent";
    }

    #[test]
    fn e2e_filesystem_export() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::now_v7();
        let ctx = Context::try_new(id).unwrap();
        let options = ExporterOptions::FileSystem(FileSystemExporterOptions::new(
            FileSystemFormat::Ndjson,
            dir.path().to_path_buf(),
        ));
        write_sidecar(&options, id, TestModel::model_info());

        let context_dir = dir.path().join(id.to_string());

        {
            let observer = ctx
                .block_on(async { ctx.observer::<TestEvent>(options).await })
                .unwrap();
            observer.send(Event::new_now(Uuid::now_v7(), TestEvent));
            // Drop the observer to drain and flush before asserting.
        }

        assert!(
            context_dir.join("model.qmi").is_file(),
            "sidecar should sit in the context directory"
        );
        let ndjson_files: Vec<_> = std::fs::read_dir(context_dir.join("TestEvent"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("ndjson"))
            .collect();
        assert_eq!(
            ndjson_files.len(),
            1,
            "one UUID-named ndjson batch file in the entity subdirectory"
        );
    }
}
