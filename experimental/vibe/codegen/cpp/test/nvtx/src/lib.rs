// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
#[allow(unused, clippy::all)]
mod bridge {
    include!(concat!(env!("OUT_DIR"), "/bridge_mod.rs"));
}

#[cfg(all(test, not(all(target_os = "linux", target_pointer_width = "64"))))]
#[test]
#[ignore = "nvtx-injection and the compiled capture fixture require Linux64"]
fn compiled_nvtx_capture_requires_linux64() {
    panic!("run the compiled NVTX capture fixture on Linux64; this host cannot capture NVTX");
}

#[cfg(all(test, target_os = "linux", target_pointer_width = "64"))]
mod tests {
    use std::{ffi::CString, path::Path, process::Command};

    use quent_instrumentation::Event;
    use quent_io::{FileSystemFormat, ImporterProvider};
    use quent_nvtx_events::{NvtxEvent, NvtxMessage};
    use uuid::Uuid;

    unsafe extern "C" {
        fn quent_nvtx_cpp_run(
            scenario: *const std::ffi::c_char,
            root: *const std::ffi::c_char,
        ) -> std::ffi::c_int;
    }

    // Never run two hook scenarios in one process: even a dropped hook cannot
    // be installed again. Separate subprocesses also reset NVTX initialization.
    fn subprocess(scenario: &str) {
        let directory = tempfile::tempdir().unwrap();
        let result = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::capture_child", "--nocapture"])
            .env("QUENT_NVTX_SCENARIO", scenario)
            .env("QUENT_NVTX_OUTPUT", directory.path())
            // This fixture uses the static attach path, never an external tool.
            .env_remove("NVTX_INJECTION64_PATH")
            .env_remove("NVTX_INJECTION32_PATH")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{scenario} subprocess failed:\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr),
        );
    }

    #[test]
    fn disabled_and_noop_do_not_capture_or_consume_installation() {
        subprocess("disabled");
    }

    #[test]
    fn filesystem_capture_roundtrips_and_flushes_on_drop() {
        for format in ["ndjson", "msgpack", "postcard"] {
            subprocess(format);
        }
    }

    #[test]
    fn raw_bridge_can_enable_capture() {
        subprocess("raw");
    }

    #[test]
    fn duplicate_installation_preserves_first_capture() {
        subprocess("duplicate");
    }

    #[test]
    fn context_drop_stops_capture_with_a_surviving_schema_observer() {
        subprocess("retained");
    }

    #[test]
    fn capture_child() {
        let Ok(scenario) = std::env::var("QUENT_NVTX_SCENARIO") else {
            return;
        };
        let root = std::env::var_os("QUENT_NVTX_OUTPUT").unwrap();
        let root = Path::new(&root);
        let scenario_c = CString::new(scenario.as_str()).unwrap();
        let root_c = CString::new(root.to_str().unwrap()).unwrap();
        assert_eq!(
            unsafe { quent_nvtx_cpp_run(scenario_c.as_ptr(), root_c.as_ptr()) },
            0,
            "C++ scenario {scenario} failed",
        );

        let capture_root = if scenario == "disabled" {
            for name in ["default", "explicit", "raw"] {
                assert!(
                    !contains_stream(&root.join(name), "NvtxEvent"),
                    "disabled {name} created an NVTX exporter"
                );
            }
            root.join("enabled")
        } else {
            root.to_path_buf()
        };
        let id: Uuid = std::fs::read_to_string(capture_root.join("context-id"))
            .unwrap()
            .parse()
            .unwrap();
        let format = match scenario.as_str() {
            "msgpack" => FileSystemFormat::Msgpack,
            "postcard" => FileSystemFormat::Postcard,
            _ => FileSystemFormat::Ndjson,
        };
        let context_dir = capture_root.join(id.to_string());
        let events: Vec<Event<NvtxEvent>> = quent_io::filesystem::importer::Options {
            format,
            path: context_dir.join("NvtxEvent"),
        }
        .create_importer()
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
        let expected = if scenario == "duplicate" { 6 } else { 5 };
        assert_eq!(
            events.len(),
            expected,
            "missing, unflushed, or out-of-window NVTX events: {events:?}"
        );
        assert!(
            events.iter().all(|event| event.id == id),
            "NVTX event IDs differ from the owning context"
        );
        assert!(matches!(&events[0].data, NvtxEvent::Mark { .. }));
        assert_message(&events[0].data, "captured-mark");
        assert_message(&events[1].data, "captured-push");
        assert_message(&events[3].data, "captured-start");
        let NvtxEvent::RangePush {
            thread_id, domain, ..
        } = &events[1].data
        else {
            panic!("expected push");
        };
        assert!(matches!(&events[2].data,
            NvtxEvent::RangePop { thread_id: popped, domain: popped_domain }
            if popped == thread_id && popped_domain == domain));
        let NvtxEvent::RangeStart {
            range_id, domain, ..
        } = &events[3].data
        else {
            panic!("expected start");
        };
        assert!(matches!(&events[4].data,
            NvtxEvent::RangeEnd { range_id: ended, domain: ended_domain }
            if ended == range_id && ended_domain == domain));
        if scenario == "duplicate" {
            assert!(matches!(&events[5].data, NvtxEvent::Mark { .. }));
            assert_message(&events[5].data, "first-still-active");
        }
        if matches!(
            scenario.as_str(),
            "ndjson" | "msgpack" | "postcard" | "duplicate" | "retained"
        ) {
            assert!(
                context_dir.join("Cluster").is_dir(),
                "schema exporter must share the NVTX context directory"
            );
            if matches!(format, FileSystemFormat::Ndjson) {
                let schema_events: Vec<Event<serde_json::Value>> =
                    quent_io::filesystem::importer::Options {
                        format,
                        path: context_dir.join("Cluster"),
                    }
                    .create_importer()
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap();
                assert_eq!(schema_events.len(), 1);
                assert_eq!(schema_events[0].id, id);
                assert!(schema_events[0].data.to_string().contains("nvtx-fixture"));
            }
        }
    }

    fn assert_message(event: &NvtxEvent, expected: &str) {
        let attributes = match event {
            NvtxEvent::Mark { attributes, .. }
            | NvtxEvent::RangePush { attributes, .. }
            | NvtxEvent::RangeStart { attributes, .. } => attributes,
            other => panic!("expected a named annotation, got {other:?}"),
        };
        assert_eq!(
            attributes.message,
            Some(NvtxMessage::String(expected.to_owned()))
        );
    }

    fn contains_stream(path: &Path, name: &str) -> bool {
        path.is_dir()
            && std::fs::read_dir(path).unwrap().any(|entry| {
                let path = entry.unwrap().path();
                path.file_name().is_some_and(|value| value == name)
                    || (path.is_dir() && contains_stream(&path, name))
            })
    }
}
