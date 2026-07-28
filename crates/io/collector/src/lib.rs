// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter sending events to a Collector service

use quent_collector_client::Client;
use quent_events::{EntityEvent, Event};
use quent_io_types::{Exporter, ExporterError, ExporterProvider, ExporterResult};
use serde::Serialize;
use uuid::Uuid;

/// User-facing options for the collector exporter.
///
/// Streams events over gRPC to a remote collector service.
#[derive(Debug, Default, Clone)]
pub struct Options {
    address: http::Uri,
}

impl Options {
    pub fn new(address: http::Uri) -> Self {
        Self { address }
    }
}

#[async_trait::async_trait]
impl<T> ExporterProvider<T> for Options
where
    T: Send + EntityEvent + 'static,
    T: serde::Serialize,
{
    async fn create_exporter(&self, context_id: Uuid) -> ExporterResult<Box<dyn Exporter<T>>> {
        Ok(Box::new(
            CollectorExporter::<T>::try_new(self.address.clone(), context_id)
                .await
                .map_err(ExporterError::Other)?,
        ) as Box<dyn Exporter<T>>)
    }
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
    async fn shutdown(mut self: Box<Self>) -> ExporterResult<()> {
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
