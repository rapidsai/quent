// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

mod browser {
    pub struct Summary {
        pub engine_id: quent_instrumentation::Uuid,
        pub context_count: usize,
        pub event_count: usize,
        pub schema_digest: String,
        pub format_version: u32,
    }

    pub fn generate_bundle(
        _input: &std::path::Path,
        _output: &std::path::Path,
    ) -> Result<Summary, std::io::Error> {
        Ok(Summary {
            engine_id: quent_instrumentation::Uuid::nil(),
            context_count: 0,
            event_count: 0,
            schema_digest: String::new(),
            format_version: 1,
        })
    }
}

include!(concat!(env!("OUT_DIR"), "/model.rs"));
include!(concat!(env!("OUT_DIR"), "/pyo3_bridge.rs"));
