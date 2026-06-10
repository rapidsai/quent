// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Server code for the Collector service.
//!
//! This allows multiple sources to send events to a centralized place, where it can be further processed / exported.

pub mod server;

/// Re-exported so callers constructing a [`server::CollectorService`] can name
/// the `T: ModelSource` bound it requires without depending on `quent-exporter`.
pub use quent_exporter::ModelSource;
