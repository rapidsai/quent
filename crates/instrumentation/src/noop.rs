// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! No-op event export.

use quent_build_info::ModelInfo;
use quent_events::Event;
use quent_io::{Exporter, ExporterProvider, ExporterResult};
use uuid::Uuid;

use crate::ContextExporter;

/// An exporter provider that discards every event.
#[derive(Clone, Copy, Debug, Default)]
pub struct Noop;

#[async_trait::async_trait]
impl<T> Exporter<T> for Noop
where
    T: Send + 'static,
{
    async fn push(&mut self, _event: Event<T>) -> ExporterResult<()> {
        Ok(())
    }

    async fn shutdown(self: Box<Self>) -> ExporterResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl<T> ExporterProvider<T> for Noop
where
    T: Send + 'static,
{
    async fn create_exporter(&self, _context_id: Uuid) -> ExporterResult<Box<dyn Exporter<T>>> {
        Ok(Box::new(*self))
    }
}

impl ContextExporter for Noop {
    fn is_noop(&self) -> bool {
        true
    }

    fn prepare_context(&self, _context_id: Uuid, _model: ModelInfo) {}
}
