// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared fixtures for the instrumentation integration tests.

#![allow(dead_code)]

use quent_build_info::{BuildInfo, ModelSource};
use quent_events::EntityEvent;
use serde::{Deserialize, Serialize};

/// Model marker, only used to supply provenance to `write_sidecar`.
pub struct TestModel;

impl ModelSource for TestModel {
    fn package() -> &'static str {
        "quent-instrumentation-tests"
    }
    fn source() -> BuildInfo {
        BuildInfo::unknown()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestEvent;

impl EntityEvent for TestEvent {
    const NAME: &'static str = "TestEvent";
}
