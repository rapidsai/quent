// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#![cfg(all(target_os = "linux", target_pointer_width = "64"))]

#[allow(unused)]
mod instrumentation {
    include!(concat!(env!("OUT_DIR"), "/nvtx_cpp_test.rs"));
}

#[allow(unused, clippy::all)]
mod bridge {
    include!(concat!(env!("OUT_DIR"), "/bridge_mod.rs"));
}

#[cfg(test)]
mod tests {
    use quent_events::EventPayload;
    use quent_io::{ImporterOptions, ImporterProvider, filesystem};
    use quent_nvtx_sink::quent::nvtx::SinkEvent;
    use uuid::Uuid;

    unsafe extern "C" {
        fn quent_nvtx_cpp_run(output: *const std::ffi::c_char) -> std::ffi::c_int;
    }

    #[test]
    fn cpp_capture_uses_the_schema_observer_and_owns_its_lifetime() {
        let output = tempfile::tempdir().unwrap();
        let path = std::ffi::CString::new(output.path().to_str().unwrap()).unwrap();
        // SAFETY: The C++ call borrows a valid NUL-terminated path for its duration.
        let result = unsafe { quent_nvtx_cpp_run(path.as_ptr()) };
        assert_eq!(
            result,
            0,
            "{:?}",
            std::fs::read_to_string(output.path().join("error"))
        );
        let context: Uuid = std::fs::read_to_string(output.path().join("context-id"))
            .unwrap()
            .parse()
            .unwrap();
        let process: Uuid = std::fs::read_to_string(output.path().join("process-id"))
            .unwrap()
            .parse()
            .unwrap();
        let importer = <ImporterOptions as ImporterProvider<SinkEvent>>::create_importer(
            &ImporterOptions::FileSystem(filesystem::importer::Options {
                format: filesystem::Format::Ndjson,
                path: output
                    .path()
                    .join(context.to_string())
                    .join(SinkEvent::NAME),
            }),
        )
        .unwrap();
        let events: Vec<_> = importer.collect::<Result<_, _>>().unwrap();
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|event| event.id == events[0].id));
        assert_ne!(events[0].id, process);
        assert!(
            matches!(&events[0].data, SinkEvent::Initialized { process: reference } if reference.target == process)
        );
        let names: Vec<_> = events
            .iter()
            .filter_map(|event| match &event.data {
                SinkEvent::Mark { data } => data.attributes.message.as_ref()?.string.as_deref(),
                _ => None,
            })
            .collect();
        assert_eq!(names, ["before-context-drop", "after-context-drop"]);
    }
}
