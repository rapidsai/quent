// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Model provenance sidecar written into the filesystem exporter directory.

use quent_build_info::{ArtifactInfo, ModelInfo};
use quent_io::ExporterOptions;
use tracing::warn;
use uuid::Uuid;

/// Prepares context-wide output for an exporter provider.
pub trait ContextExporter {
    /// Returns whether this provider disables exporter pipelines.
    fn is_noop(&self) -> bool {
        false
    }

    /// Prepares output for `model` under `context_id`.
    fn prepare_context(&self, context_id: Uuid, model: ModelInfo);
}

impl ContextExporter for ExporterOptions {
    fn prepare_context(&self, context_id: Uuid, model: ModelInfo) {
        write_sidecar(self, context_id, model);
    }
}

#[cfg(feature = "io-callback")]
impl<T> ContextExporter for quent_io_callback::EventCallback<T> {
    fn prepare_context(&self, _context_id: Uuid, _model: ModelInfo) {}
}

/// Write the model provenance sidecar file into the filesystem exporter
/// directory.
///
/// If the options do not target a filesystem exporter, then this is a no-op.
pub fn write_sidecar(options: &ExporterOptions, context_id: Uuid, model: ModelInfo) {
    let Some(root) = options.filesystem_root(context_id) else {
        return;
    };
    if let Err(e) =
        std::fs::create_dir_all(&root).and_then(|()| ArtifactInfo::new(model).write_sidecar(&root))
    {
        warn!("failed to write provenance sidecar: {e}");
    }
}
