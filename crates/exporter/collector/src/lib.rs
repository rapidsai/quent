// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Exporter sending events to a Collector service

use quent_collector_client::Client;
use quent_events::Event;
use quent_exporter_types::{Exporter, ExporterError, ExporterResult};
use serde::Serialize;
use uuid::Uuid;

/// Options for the collector exporter.
///
/// Streams events over gRPC to a remote collector service. Use this for
/// distributed deployments where events are centralized for analysis.
#[derive(Debug, Default, Clone)]
pub struct CollectorExporterOptions {
    pub address: String,
}

#[derive(Debug)]
pub struct CollectorExporter<T> {
    client: Client<T>,
}

impl<T> CollectorExporter<T>
where
    T: Serialize + Send + 'static,
{
    pub async fn try_new(
        application_id: Uuid,
        options: CollectorExporterOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::new(application_id, options.address).await?;
        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl<T> Exporter<T> for CollectorExporter<T>
where
    T: Serialize + Send + 'static,
{
    async fn push(&self, event: Event<T>) -> ExporterResult<()> {
        self.client
            .send(event)
            .await
            .map_err(|e| ExporterError::Collector(format!("{e:?}")))?;
        Ok(())
    }
    async fn force_flush(&self) -> ExporterResult<()> {
        // TODO(johanpel): figure this out, it may be that we don't need this trait fn
        Ok(())
    }
}
