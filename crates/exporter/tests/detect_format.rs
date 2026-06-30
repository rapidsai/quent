// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Exercises the msgpack filesystem exporter; a `--no-default-features` build has
// neither it nor `FileSystemFormat::detect` (both gated on the `filesystem` cfg).
#![cfg(feature = "msgpack")]

//! `FileSystemFormat::detect` recognizes the format of a context written by the
//! real exporter, rather than a hand-fabricated `events.<ext>` layout the exporter
//! never actually produces.

use quent_exporter::{
    FileSystemExporterOptions, FileSystemFormat, ResolvedExporterOptions, create_exporter,
};
use quent_query_engine_model::engine::EngineEvent;

#[tokio::test]
async fn detects_format_from_a_real_exporter_write() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // The real per-observer exporter lays out `<root>/<entity>/<uuid>.<ext>`;
    // creating it writes the stream file. Drive that rather than assuming a name.
    let _exporter = create_exporter::<EngineEvent>(ResolvedExporterOptions::FileSystem(
        FileSystemExporterOptions {
            format: FileSystemFormat::Msgpack,
            root: root.to_path_buf(),
        },
    ))
    .await
    .expect("create filesystem exporter");

    assert_eq!(
        FileSystemFormat::detect(root),
        Some(FileSystemFormat::Msgpack)
    );
}
