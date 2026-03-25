// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Basic traits for exporter / importer implementations

use quent_events::Event;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExporterError {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("flush error: {0}")]
    Flush(String),
    #[error("serde error: {0}")]
    Serde(String),
    #[error("collector error: {0}")]
    Collector(String),
}

#[derive(Error, Debug)]
pub enum ImporterError {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type ExporterResult<T> = std::result::Result<T, ExporterError>;
pub type ImporterResult<T> = std::result::Result<T, ImporterError>;

#[async_trait::async_trait]
pub trait Exporter<T>: Send + Sync + std::fmt::Debug
where
    T: Serialize + Send,
{
    async fn push(&self, event: Event<T>) -> ExporterResult<()>;
    async fn force_flush(&self) -> ExporterResult<()>;
}

pub trait Importer<T>: Iterator<Item = Event<T>>
where
    T: for<'de> Deserialize<'de>,
{
}
