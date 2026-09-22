// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Context and source partitioning for generated canonical NVTX streams.

use nvtx_analyzer::{
    NvtxEventData, NvtxEventView, NvtxProcessBindingData, NvtxSourceError, NvtxSourcesBuilder,
};
use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};
use quent_events::Event;
use uuid::Uuid;

#[derive(Debug)]
enum StoredNvtxEvent {
    Initialized { process: Uuid },
    Captured(NvtxEvent),
}

impl NvtxProcessBindingData for StoredNvtxEvent {
    fn nvtx_process_id(&self) -> Option<Uuid> {
        match self {
            Self::Initialized { process } => Some(*process),
            Self::Captured(_) => None,
        }
    }
}

impl NvtxEventData for StoredNvtxEvent {
    fn nvtx_event(&self) -> Option<NvtxEventView<'_>> {
        match self {
            Self::Initialized { .. } => None,
            Self::Captured(event) => event.nvtx_event(),
        }
    }
}

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn process_id(data: impl NvtxProcessBindingData) -> Option<Uuid> {
    data.nvtx_process_id()
}

fn at(stream_id: Uuid, timestamp: u64, data: StoredNvtxEvent) -> Event<StoredNvtxEvent> {
    Event::new(stream_id, timestamp, data)
}

fn initialized(stream_id: Uuid, timestamp: u64, process: Uuid) -> Event<StoredNvtxEvent> {
    at(
        stream_id,
        timestamp,
        StoredNvtxEvent::Initialized { process },
    )
}

fn start(stream_id: Uuid, timestamp: u64, name: &str) -> Event<StoredNvtxEvent> {
    at(
        stream_id,
        timestamp,
        StoredNvtxEvent::Captured(NvtxEvent::RangeStart {
            domain: 7,
            range_id: 11,
            attributes: NvtxEventAttributes {
                message: Some(NvtxMessage::String(name.to_owned())),
                ..Default::default()
            },
        }),
    )
}

fn end(stream_id: Uuid, timestamp: u64) -> Event<StoredNvtxEvent> {
    at(
        stream_id,
        timestamp,
        StoredNvtxEvent::Captured(NvtxEvent::RangeEnd {
            domain: 7,
            range_id: 11,
        }),
    )
}

#[test]
fn isolates_overlapping_raw_ids_by_context_and_stream() {
    let context_a = id(1);
    let context_b = id(2);
    let stream_a = id(10);
    let stream_b = id(20);
    let process_a = id(100);
    let process_b = id(200);
    let process_c = id(300);

    let mut builder = NvtxSourcesBuilder::new();
    // Deliberately interleaved and reverse ordered. Every source reuses the
    // same domain and range id, so any cross-source reconstruction would pair
    // the wrong start/end or overwrite a range.
    builder.push(context_b, end(stream_a, 70));
    builder.push(context_a, start(stream_b, 30, "a/stream-b"));
    builder.push(context_a, initialized(stream_a, 1, process_a));
    builder.push(context_b, initialized(stream_a, 3, process_c));
    builder.push(context_a, end(stream_a, 20));
    builder.push(context_a, initialized(stream_b, 2, process_b));
    builder.push(context_b, start(stream_a, 50, "b/stream-a"));
    builder.push(context_a, start(stream_a, 10, "a/stream-a"));
    builder.push(context_a, end(stream_b, 40));

    let sources = builder.build().expect("valid sources");
    let identities: Vec<_> = sources
        .iter()
        .map(|source| (source.context_id(), source.process_id(), source.stream_id()))
        .collect();
    assert_eq!(
        identities,
        [
            (context_a, process_a, stream_a),
            (context_a, process_b, stream_b),
            (context_b, process_c, stream_a),
        ]
    );

    let names: Vec<_> = sources
        .iter()
        .map(|source| {
            let spans = source.model().spans();
            assert_eq!(spans.len(), 1, "one range belongs to each source");
            spans[0].name.as_str()
        })
        .collect();
    assert_eq!(names, ["a/stream-a", "a/stream-b", "b/stream-a"]);
}

#[test]
fn initialization_metadata_contributes_to_trace_bounds() {
    let context = id(1);
    let stream = id(2);
    let process = id(3);
    let binding = StoredNvtxEvent::Initialized { process };
    assert_eq!(process_id(&binding), Some(process));

    let mut builder = NvtxSourcesBuilder::new();
    builder.extend(
        context,
        [
            start(stream, 20, "bounded"),
            initialized(stream, 5, process),
            end(stream, 30),
        ],
    );

    let source = builder.build().unwrap().pop().unwrap();
    assert_eq!(source.model().trace_start(), 5);
    assert_eq!(source.model().trace_end(), 30);
    assert_eq!(source.model().spans().len(), 1);

    let later_context = id(4);
    let later_stream = id(5);
    let mut builder = NvtxSourcesBuilder::new();
    builder.extend(
        later_context,
        [
            end(later_stream, 30),
            initialized(later_stream, 40, process),
            start(later_stream, 20, "bounded at the end"),
        ],
    );
    let source = builder.build().unwrap().pop().unwrap();
    assert_eq!(source.model().trace_start(), 20);
    assert_eq!(source.model().trace_end(), 40);
}

#[test]
fn missing_process_binding_is_rejected() {
    let context = id(1);
    let stream = id(2);
    let mut builder = NvtxSourcesBuilder::new();
    builder.push(context, start(stream, 10, "unbound"));

    assert_eq!(
        builder.build().unwrap_err(),
        NvtxSourceError::MissingProcessBinding {
            context_id: context,
            stream_id: stream,
        }
    );
}

#[test]
fn conflicting_process_bindings_are_rejected_deterministically() {
    let context = id(1);
    let stream = id(2);
    let lower = id(10);
    let higher = id(20);
    let mut builder = NvtxSourcesBuilder::new();
    builder.extend(
        context,
        [
            initialized(stream, 1, higher),
            initialized(stream, 2, lower),
        ],
    );

    assert_eq!(
        builder.build().unwrap_err(),
        NvtxSourceError::ConflictingProcessBindings {
            context_id: context,
            stream_id: stream,
            process_ids: vec![lower, higher],
        }
    );
}

#[test]
fn duplicate_process_binding_is_rejected() {
    let context = id(1);
    let stream = id(2);
    let process = id(3);
    let mut builder = NvtxSourcesBuilder::new();
    builder.extend(
        context,
        [
            initialized(stream, 1, process),
            initialized(stream, 2, process),
        ],
    );

    assert_eq!(
        builder.build().unwrap_err(),
        NvtxSourceError::DuplicateProcessBinding {
            context_id: context,
            stream_id: stream,
            process_id: process,
            occurrences: 2,
        }
    );
}
