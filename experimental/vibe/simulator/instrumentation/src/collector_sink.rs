// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Collector ingest for the simulator model plus the NVTX side-stream.
//!
//! NVTX is not a `model!` entity, so [`SimulatorContext`] rejects `NvtxEvent`.
//! This sink mirrors that stream onto the same context id the analyzer reads.

use nvtx_bridge::NvtxEventEntity;
use quent_model::{
    CollectorSink, ContextInner, EntityEvent, Observer, deserialize_event, io::ExporterOptions,
};
use uuid::Uuid;

use crate::SimulatorContext;

/// Per-source collector sink: query-engine entities plus [`NvtxEventEntity`].
pub struct SimulatorCollectorSink {
    context: SimulatorContext,
    nvtx: Observer<NvtxEventEntity>,
}

impl SimulatorCollectorSink {
    /// Reproduce one remote source under `id`, including its NVTX stream.
    pub fn try_new(
        id: Uuid,
        provider: ExporterOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let context = SimulatorContext::try_with_id(id, provider.clone())?;
        let inner = ContextInner::try_new(id)?;
        let nvtx = inner.block_on(async { inner.observer::<NvtxEventEntity>(&provider).await })?;
        Ok(Self { context, nvtx })
    }
}

impl CollectorSink for SimulatorCollectorSink {
    fn ingest(&self, entity: &str, event: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if entity == NvtxEventEntity::NAME {
            self.nvtx.send(deserialize_event::<NvtxEventEntity>(event)?);
            return Ok(());
        }
        self.context.ingest(entity, event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvtx_events::NvtxEvent;
    use quent_model::{
        Event,
        io::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat},
    };

    #[test]
    fn ingest_persists_nvtx_beside_the_context() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::now_v7();
        let provider = ExporterOptions::FileSystem(FileSystemExporterOptions::new(
            FileSystemFormat::Ndjson,
            dir.path().to_path_buf(),
        ));
        let event = Event::new(
            id,
            1,
            NvtxEventEntity(NvtxEvent::DomainCreate {
                domain: 1,
                name: "CCCL".into(),
            }),
        );
        let bytes = bitcode::serialize(&event).unwrap();
        {
            let sink = SimulatorCollectorSink::try_new(id, provider).unwrap();
            sink.ingest(NvtxEventEntity::NAME, &bytes).unwrap();
        }
        let ndjson: Vec<_> =
            std::fs::read_dir(dir.path().join(id.to_string()).join(NvtxEventEntity::NAME))
                .unwrap()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("ndjson"))
                .collect();
        assert_eq!(ndjson.len(), 1);
        let body = std::fs::read_to_string(&ndjson[0]).unwrap();
        assert!(body.contains("CCCL"), "flushed NVTX event: {body}");
    }
}
