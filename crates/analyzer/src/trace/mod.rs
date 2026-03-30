// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use rustc_hash::FxHashMap as HashMap;

use quent_attributes::Attribute;
use quent_events::trace::{SpanId, TraceEvent};
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{AnalyzerError, AnalyzerResult, Entity, Span};

/// A run-time defined tracing span with active intervals and child span ids.
#[derive(Debug)]
pub struct RtSpan {
    /// The ID of this span.
    pub id: SpanId,
    /// The name of this span.
    pub name: String,
    /// The parent ID of this spann.
    pub parent_id: Option<SpanId>,
    /// The span of time of this span, incl
    pub span: SpanUnixNanoSec,
    /// The intervals at which this span was active.
    pub intervals: SmallVec<[SpanUnixNanoSec; 1]>,
    /// The children of this span.
    pub children: Vec<SpanId>,
    /// The attributes of this span.
    pub attributes: Vec<Attribute>,
}

/// The reconstructed span tree from trace events.
#[derive(Debug)]
pub struct RtTrace {
    pub id: Uuid,
    pub instance_name: String,
    pub span: SpanUnixNanoSec,
    pub spans: HashMap<SpanId, RtSpan>,
    pub roots: Vec<SpanId>,
}

/// A timestamped trace event.
struct EventWithTs {
    timestamp: TimeUnixNanoSec,
    event: TraceEvent,
}

/// Builder that reconstructs the span tree from a stream of trace events.
///
/// Events are inserted in timestamp order.
#[derive(Default)]
pub struct RtTraceBuilder {
    id: Uuid,
    /// Common case is that timestamps will arrive in-order, so we typically
    /// insert with a push back. Common case is also that we just init, enter,
    /// exit, close.
    // TODO(johanpel): consider making this just 2 events, and move the other
    // two actions to the instrumentation lib.
    events: SmallVec<[EventWithTs; 4]>,
}

impl RtTraceBuilder {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "trace id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                events: SmallVec::new(),
            })
        }
    }

    pub fn push(&mut self, timestamp: TimeUnixNanoSec, event: TraceEvent) {
        if self
            .events
            .last()
            .is_none_or(|last| last.timestamp <= timestamp)
        {
            self.events.push(EventWithTs { timestamp, event });
        } else {
            let pos = self.events.partition_point(|e| e.timestamp <= timestamp);
            self.events.insert(pos, EventWithTs { timestamp, event });
        }
    }

    pub fn try_build(self) -> AnalyzerResult<RtTrace> {
        let mut entries: HashMap<SpanId, RtSpanBuilder> = HashMap::default();

        for e in self.events {
            match e.event {
                TraceEvent::Init(_) => {}
                TraceEvent::Span(init) => {
                    if let Some(parent_id) = init.parent_id {
                        entries.entry(parent_id).or_default().children.push(init.id);
                    }
                    let entry = entries.entry(init.id).or_default();
                    entry.name = init.name;
                    entry.parent_id = init.parent_id;
                    entry.init_attributes = init.attributes;
                }
                TraceEvent::Enter(enter) => {
                    let entry = entries.entry(enter.id).or_default();
                    entry.pending_enter = Some((e.timestamp, enter.attributes));
                }
                TraceEvent::Exit(exit) => {
                    let entry = entries.entry(exit.id).or_default();
                    let (enter_ts, enter_attrs) = entry.pending_enter.take().ok_or_else(|| {
                        AnalyzerError::Validation(format!(
                            "span exit without matching enter for span id {}",
                            exit.id
                        ))
                    })?;
                    entry
                        .intervals
                        .push(SpanUnixNanoSec::try_new(enter_ts, e.timestamp)?);
                    entry.attributes.extend(enter_attrs);
                    entry.attributes.extend(exit.attributes);
                }
                TraceEvent::Close(close) => {
                    let entry = entries.entry(close.id).or_default();
                    entry.close_attributes = close.attributes;
                }
            }
        }

        // Build spans, computing cached time spans from intervals.
        let mut roots = Vec::new();
        let mut trace_start: Option<TimeUnixNanoSec> = None;
        let mut trace_end: Option<TimeUnixNanoSec> = None;

        let spans: HashMap<SpanId, RtSpan> = entries
            .into_iter()
            .map(|(id, entry)| {
                let span = span_from_intervals(&entry.intervals, id)?;

                if entry.parent_id.is_none() {
                    roots.push(id);
                    trace_start =
                        Some(trace_start.map_or(span.start(), |cur| cur.min(span.start())));
                    trace_end = Some(trace_end.map_or(span.end(), |cur| cur.max(span.end())));
                }

                // Merge all attributes from init, enter/exit, and close.
                let mut attributes = entry.init_attributes;
                attributes.extend(entry.attributes);
                attributes.extend(entry.close_attributes);

                Ok((
                    id,
                    RtSpan {
                        id,
                        name: entry.name,
                        parent_id: entry.parent_id,
                        span,
                        intervals: entry.intervals,
                        children: entry.children,
                        attributes,
                    },
                ))
            })
            .collect::<AnalyzerResult<_>>()?;

        let trace_span = SpanUnixNanoSec::try_new(
            trace_start
                .ok_or_else(|| AnalyzerError::IncompleteEntity("trace has no root spans".into()))?,
            trace_end.unwrap(),
        )?;

        let instance_name = if roots.is_empty() {
            "(empty trace)".to_owned()
        } else {
            let first = &spans[roots.first().unwrap()].name;
            if roots.len() == 1 {
                first.clone()
            } else {
                format!("{first} (+ {} others)", roots.len() - 1)
            }
        };

        Ok(RtTrace {
            id: self.id,
            instance_name,
            span: trace_span,
            spans,
            roots,
        })
    }
}

fn span_from_intervals(
    intervals: &[SpanUnixNanoSec],
    span_id: SpanId,
) -> AnalyzerResult<SpanUnixNanoSec> {
    let first = intervals.first().ok_or_else(|| {
        AnalyzerError::IncompleteEntity(format!("span {} has no intervals", span_id))
    })?;
    let last = intervals.last().unwrap();
    Ok(SpanUnixNanoSec::try_new(first.start(), last.end())?)
}

#[derive(Default)]
struct RtSpanBuilder {
    name: String,
    parent_id: Option<SpanId>,
    pending_enter: Option<(TimeUnixNanoSec, Vec<Attribute>)>,
    intervals: SmallVec<[SpanUnixNanoSec; 1]>,
    children: Vec<SpanId>,
    init_attributes: Vec<Attribute>,
    attributes: Vec<Attribute>,
    close_attributes: Vec<Attribute>,
}

impl Span for RtSpan {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        Ok(self.span)
    }
}

impl Span for RtTrace {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        Ok(self.span)
    }
}

impl Entity for RtTrace {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "trace"
    }
    fn instance_name(&self) -> &str {
        &self.instance_name
    }
}
