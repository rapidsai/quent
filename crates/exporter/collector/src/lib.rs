// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter sending events to a Collector service

use quent_collector_client::Client;
use quent_events::{EntityEvent, Event};
use quent_exporter_types::{Exporter, ExporterError, ExporterResult};
use serde::Serialize;
use uuid::Uuid;

/// User-facing options for the collector exporter.
///
/// Streams events over gRPC to a remote collector service. Use this for
/// distributed deployments where events are centralized for analysis. The
/// source context id the collector reproduces under is supplied separately by
/// the context when it builds the exporter (see [`CollectorExporter::try_new`]).
#[derive(Debug, Default, Clone)]
pub struct CollectorExporterOptions {
    pub address: http::Uri,
}

/// Streams one entity's events to a collector. The stream is tagged with the
/// entity name (`T::NAME`) so the collector routes each batch to the matching
/// entity observer.
#[derive(Debug)]
pub struct CollectorExporter<T> {
    /// `None` once [`shutdown`](Exporter::shutdown) has drained and released it.
    client: Option<Client<T>>,
}

impl<T> CollectorExporter<T>
where
    T: Serialize + Send + EntityEvent + 'static,
{
    /// `source_context_id` identifies this stream to the collector, which
    /// reproduces the source's output under that id.
    pub async fn try_new(
        address: http::Uri,
        source_context_id: Uuid,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new(source_context_id, T::NAME, address).await?;
        Ok(Self {
            client: Some(client),
        })
    }
}

#[async_trait::async_trait]
impl<T> Exporter<T> for CollectorExporter<T>
where
    T: Serialize + Send + EntityEvent + 'static,
{
    async fn push(&mut self, event: Event<T>) -> ExporterResult<()> {
        let client = self.client.as_ref().ok_or(ExporterError::Shutdown)?;
        client.send(event).await.map_err(ExporterError::other)?;
        Ok(())
    }
    async fn shutdown(&mut self) -> ExporterResult<()> {
        let Some(mut client) = self.client.take() else {
            return Ok(());
        };
        // Drain buffered events and wait for delivery. The forwarder awaits this
        // on shutdown, so the client's tasks are joined here rather than in
        // `Client::drop` (which may run on a runtime worker, where blocking
        // panics).
        client.shutdown().await;
        Ok(())
    }
}
