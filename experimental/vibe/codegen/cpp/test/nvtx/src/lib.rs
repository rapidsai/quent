// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[allow(unused, clippy::all)]
mod bridge {
    include!(concat!(env!("OUT_DIR"), "/bridge_mod.rs"));
}

#[cfg(all(test, target_os = "linux", target_pointer_width = "64"))]
mod tests {
    use std::{ffi::CString, path::Path};

    use serde_json::Value;

    unsafe extern "C" {
        fn quent_codegen_cpp_nvtx_capture(
            output_dir: *const std::ffi::c_char,
            process_id: u32,
        ) -> std::ffi::c_int;
    }

    #[test]
    fn generated_cpp_api_captures_nvtx_into_its_ndjson_context() {
        let output = unique_output_dir();
        std::fs::create_dir(&output).expect("create capture root");
        let output_c = CString::new(output.to_str().expect("UTF-8 capture path")).unwrap();

        let status =
            unsafe { quent_codegen_cpp_nvtx_capture(output_c.as_ptr(), std::process::id()) };
        assert_eq!(status, 0, "C++ capture fixture failed at step {status}");

        let process_events = read_stream(&output, "Process");
        assert_eq!(process_events.len(), 1, "one successful process identity");
        let process_id = process_events[0]["id"].as_str().expect("process event id");
        assert_eq!(variant(&process_events[0]), "Started");
        assert_eq!(
            process_events[0]["data"]["Started"]["process"]["native_id"],
            Value::from(std::process::id()),
        );

        let nvtx_events = read_stream(&output, "NvtxEvent");
        assert!(
            nvtx_events.len() >= 4,
            "captured NVTX events: {nvtx_events:?}"
        );
        assert_eq!(variant(&nvtx_events[0]), "Initialized");
        assert_eq!(
            nvtx_events[0]["data"]["Initialized"]["process"]["target"],
            process_id,
        );
        assert_labeled_variant(&nvtx_events, "Mark", "cpp-generated-mark");
        assert_labeled_variant(&nvtx_events, "RangePush", "cpp-generated-range");
        assert!(
            nvtx_events.iter().any(|event| variant(event) == "RangePop"),
            "missing captured RangePop: {nvtx_events:?}",
        );

        std::fs::remove_dir_all(output).expect("remove capture root");
    }

    fn unique_output_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "quent-codegen-cpp-nvtx-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ))
    }

    fn read_stream(root: &Path, stream: &str) -> Vec<Value> {
        let context = std::fs::read_dir(root)
            .expect("read capture root")
            .map(|entry| entry.expect("read context entry").path())
            .find(|path| path.is_dir())
            .expect("generated context directory");
        let stream = context.join(stream);
        let event_file = std::fs::read_dir(&stream)
            .unwrap_or_else(|error| panic!("read stream {}: {error}", stream.display()))
            .map(|entry| entry.expect("read event entry").path())
            .find(|path| path.extension().and_then(|value| value.to_str()) == Some("ndjson"))
            .expect("NDJSON event file");
        std::fs::read_to_string(event_file)
            .expect("read NDJSON events")
            .lines()
            .map(|line| serde_json::from_str(line).expect("parse NDJSON event"))
            .collect()
    }

    fn variant(event: &Value) -> &str {
        event["data"]
            .as_object()
            .and_then(|data| data.keys().next())
            .map(String::as_str)
            .expect("externally tagged event variant")
    }

    fn assert_labeled_variant(events: &[Value], expected_variant: &str, expected_label: &str) {
        assert!(
            events.iter().any(|event| {
                variant(event) == expected_variant && contains_string(event, expected_label)
            }),
            "missing {expected_variant} labeled {expected_label:?}: {events:?}",
        );
    }

    fn contains_string(value: &Value, expected: &str) -> bool {
        match value {
            Value::String(value) => value == expected,
            Value::Array(values) => values.iter().any(|value| contains_string(value, expected)),
            Value::Object(values) => values
                .values()
                .any(|value| contains_string(value, expected)),
            _ => false,
        }
    }
}
