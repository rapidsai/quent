// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports, dead_code)]

use quent_instrumentation_build_external_test::predefined;

mod application {
    include!(concat!(env!("OUT_DIR"), "/application.rs"));
}

fn reexported_types(
    event: application::AnnotationsEvent,
    task: application::TaskEvent,
    message: application::Message,
) -> (
    predefined::AnnotationsEvent,
    predefined::TaskEvent,
    predefined::Message,
) {
    (event, task, message)
}
