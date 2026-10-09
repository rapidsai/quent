// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Analyzer entity adapters for native event storage.

use std::fmt;

use quent_events::{EntityMarker, EventPayload};
use quent_store::entity::{
    grouped::{EventStorage, NativeEntity},
    sequence::EventSequence,
};

use super::*;

/// Adapts native event storage to the analyzer's entity interface.
pub struct StoredEntity<E: EntityMarker, S: EventStorage<E>>(NativeEntity<E, S>);

impl<E: EntityMarker, S: EventStorage<E>> StoredEntity<E, S> {
    pub(crate) fn try_from_sequence(sequence: EventSequence<E>) -> AnalyzerResult<Self> {
        NativeEntity::try_from(sequence)
            .map(Self)
            .map_err(|error| quent_analyzer::AnalyzerError::Validation(error.to_string()))
    }
}

impl<E: EntityMarker, S: EventStorage<E>> std::ops::Deref for StoredEntity<E, S> {
    type Target = NativeEntity<E, S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E: EntityMarker, S: EventStorage<E>> fmt::Debug for StoredEntity<E, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<E: EntityMarker, S: EventStorage<E>> Entity for StoredEntity<E, S> {
    fn id(&self) -> Uuid {
        self.properties().id
    }
    fn type_name(&self) -> &str {
        // Resource names must match the existing resource type declarations.
        match E::Payload::NAME {
            "HostMemory" => "host_memory",
            "Storage" => "storage",
            "GpuMemory" => "gpu_memory",
            "TaskExecutorThread" => "task_executor_thread",
            "StorageChannel" => "storage_channel",
            "PcieChannel" => "pcie_channel",
            "NetworkChannel" => "network_channel",
            name => name,
        }
    }
    fn earliest_timestamp(&self) -> TimeUnixNanoSec {
        self.properties().earliest_timestamp
    }
    fn latest_timestamp(&self) -> TimeUnixNanoSec {
        self.properties().latest_timestamp
    }
}
