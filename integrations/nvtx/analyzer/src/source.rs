// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Context- and stream-aware dispatch into the source-local reconstruction core.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use quent_events::Event;
use uuid::Uuid;

use crate::{NvtxEventData, NvtxModel, NvtxModelBuilder, NvtxProcessBindingData};

/// One independently reconstructed canonical NVTX stream.
///
/// NVTX handles are local to the process capture represented by `stream_id`.
/// The context and process identities are retained so consumers can join this
/// model to application entities without merging raw NVTX identifiers.
#[derive(Debug)]
pub struct NvtxSource {
    /// Artifact context containing the stream.
    pub context_id: Uuid,
    /// Process entity referenced by the stream's `Initialized` event.
    pub process_id: Uuid,
    /// Entity id of the canonical NVTX event stream.
    pub stream_id: Uuid,
    /// Model reconstructed solely from this stream's events.
    pub model: NvtxModel,
}

impl NvtxSource {
    /// Artifact context containing this source.
    pub fn context_id(&self) -> Uuid {
        self.context_id
    }

    /// Process entity bound to this source.
    pub fn process_id(&self) -> Uuid {
        self.process_id
    }

    /// Entity id of this canonical NVTX stream.
    pub fn stream_id(&self) -> Uuid {
        self.stream_id
    }

    /// Reconstructed model for this source alone.
    pub fn model(&self) -> &NvtxModel {
        &self.model
    }
}

/// Invalid canonical NVTX stream metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NvtxSourceError {
    /// The stream has no canonical `Initialized` event.
    MissingProcessBinding {
        /// Artifact context containing the stream.
        context_id: Uuid,
        /// Entity id of the invalid stream.
        stream_id: Uuid,
    },

    /// The stream repeats the same canonical process binding.
    DuplicateProcessBinding {
        /// Artifact context containing the stream.
        context_id: Uuid,
        /// Entity id of the invalid stream.
        stream_id: Uuid,
        /// Repeated process entity id.
        process_id: Uuid,
        /// Number of binding events found.
        occurrences: usize,
    },

    /// The stream contains bindings to more than one process entity.
    ConflictingProcessBindings {
        /// Artifact context containing the stream.
        context_id: Uuid,
        /// Entity id of the invalid stream.
        stream_id: Uuid,
        /// Distinct process ids, in deterministic order.
        process_ids: Vec<Uuid>,
    },
}

impl Display for NvtxSourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProcessBinding {
                context_id,
                stream_id,
            } => write!(
                formatter,
                "NVTX stream {stream_id} in context {context_id} has no process binding"
            ),
            Self::DuplicateProcessBinding {
                context_id,
                stream_id,
                process_id,
                occurrences,
            } => write!(
                formatter,
                "NVTX stream {stream_id} in context {context_id} binds process {process_id} {occurrences} times"
            ),
            Self::ConflictingProcessBindings {
                context_id,
                stream_id,
                process_ids,
            } => write!(
                formatter,
                "NVTX stream {stream_id} in context {context_id} has conflicting process bindings {process_ids:?}"
            ),
        }
    }
}

impl std::error::Error for NvtxSourceError {}

/// Collects generated canonical NVTX events and reconstructs each source in
/// isolation.
///
/// Events are partitioned by `(context_id, event.id)`. [`Self::build`] checks
/// that every resulting stream contains exactly one process binding, then
/// returns sources ordered by `(context_id, stream_id)`.
#[derive(Debug)]
pub struct NvtxSourcesBuilder<T> {
    streams: BTreeMap<(Uuid, Uuid), Vec<Event<T>>>,
}

impl<T> Default for NvtxSourcesBuilder<T> {
    fn default() -> Self {
        Self {
            streams: BTreeMap::new(),
        }
    }
}

impl<T> NvtxSourcesBuilder<T> {
    /// Create an empty source collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one event from `context_id`.
    ///
    /// The envelope's entity id selects its stream. Input order does not select
    /// the returned source order or the replay order within a stream.
    pub fn push(&mut self, context_id: Uuid, event: Event<T>) {
        self.streams
            .entry((context_id, event.id))
            .or_default()
            .push(event);
    }

    /// Add every event in one context.
    pub fn extend(&mut self, context_id: Uuid, events: impl IntoIterator<Item = Event<T>>) {
        for event in events {
            self.push(context_id, event);
        }
    }
}

impl<T: NvtxEventData + NvtxProcessBindingData> NvtxSourcesBuilder<T> {
    /// Validate and reconstruct every collected source.
    ///
    /// Metadata events remain in the input to reconstruction. Although they do
    /// not produce NVTX records, their envelope timestamps define the capture's
    /// observation bounds.
    pub fn build(self) -> Result<Vec<NvtxSource>, NvtxSourceError> {
        let mut sources = Vec::with_capacity(self.streams.len());

        for ((context_id, stream_id), events) in self.streams {
            let process_bindings: Vec<_> = events
                .iter()
                .filter_map(|event| event.data.nvtx_process_id())
                .collect();
            let process_ids: BTreeSet<_> = process_bindings.iter().copied().collect();

            let process_id = match (process_bindings.len(), process_ids.len()) {
                (0, _) => {
                    return Err(NvtxSourceError::MissingProcessBinding {
                        context_id,
                        stream_id,
                    });
                }
                (_, distinct) if distinct > 1 => {
                    return Err(NvtxSourceError::ConflictingProcessBindings {
                        context_id,
                        stream_id,
                        process_ids: process_ids.into_iter().collect(),
                    });
                }
                (occurrences, 1) if occurrences > 1 => {
                    return Err(NvtxSourceError::DuplicateProcessBinding {
                        context_id,
                        stream_id,
                        process_id: *process_ids.first().expect("one process id"),
                        occurrences,
                    });
                }
                (1, 1) => *process_ids.first().expect("one process id"),
                _ => unreachable!("binding counts are exhaustive"),
            };

            let model = NvtxModelBuilder::build_from(
                events
                    .into_iter()
                    .map(|event| (event.timestamp, event.data)),
            );
            sources.push(NvtxSource {
                context_id,
                process_id,
                stream_id,
                model,
            });
        }

        Ok(sources)
    }
}
