// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "msgpack")]

use quent_events::EntityEvent;
use quent_io::{Exporter, ExporterProvider, filesystem};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
struct TestEvent;

impl EntityEvent for TestEvent {
    const NAME: &'static str = "TestEvent";
}

#[tokio::test]
async fn detects_format_from_exporter_output() {
    let root = tempfile::tempdir().unwrap();
    let context_id = Uuid::now_v7();
    let options =
        filesystem::exporter::Options::new(filesystem::Format::Msgpack, root.path().to_path_buf());

    let _exporter: Box<dyn Exporter<TestEvent>> =
        options.create_exporter(context_id).await.unwrap();

    assert_eq!(
        filesystem::Format::detect(&root.path().join(context_id.to_string())),
        Some(filesystem::Format::Msgpack)
    );
}
