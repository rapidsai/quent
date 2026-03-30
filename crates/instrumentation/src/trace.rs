// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use quent_attributes::Attribute;
use quent_events::{Event, trace};
use serde::Serialize;
use uuid::Uuid;

use crate::EventSender;

#[derive(Clone)]
pub struct TraceObserver<T>
where
    T: From<trace::TraceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: EventSender<T>,
    next_span_id: Arc<AtomicU64>,
}

impl<T> TraceObserver<T>
where
    T: From<trace::TraceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: EventSender<T>, entity_id: Uuid) -> Self {
        tx.send(Event::new_now(
            entity_id,
            trace::TraceEvent::Init(trace::TraceInit { entity_id }).into(),
        ));
        Self {
            tx,
            next_span_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn alloc_span_id(&self) -> trace::SpanId {
        self.next_span_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn span(&self, id: Uuid, name: String, parent_id: Option<trace::SpanId>) -> SpanHandle<T> {
        let span_id = self.alloc_span_id();
        self.tx.send(Event::new_now(
            id,
            trace::TraceEvent::Span(trace::SpanInit {
                id: span_id,
                name,
                parent_id,
                attributes: vec![],
            })
            .into(),
        ));
        SpanHandle {
            tx: self.tx.clone(),
            trace_id: id,
            span_id,
        }
    }
}

pub struct SpanHandle<T>
where
    T: From<trace::TraceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: EventSender<T>,
    trace_id: Uuid,
    span_id: trace::SpanId,
}

impl<T> SpanHandle<T>
where
    T: From<trace::TraceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn span_id(&self) -> trace::SpanId {
        self.span_id
    }

    pub fn enter(&self, attributes: Vec<Attribute>) {
        self.tx.send(Event::new_now(
            self.trace_id,
            trace::TraceEvent::Enter(trace::SpanEnter {
                id: self.span_id,
                attributes,
            })
            .into(),
        ))
    }

    pub fn exit(&self, attributes: Vec<Attribute>) {
        self.tx.send(Event::new_now(
            self.trace_id,
            trace::TraceEvent::Exit(trace::SpanExit {
                id: self.span_id,
                attributes,
            })
            .into(),
        ))
    }

    pub fn close(self) {
        self.tx.send(Event::new_now(
            self.trace_id,
            trace::TraceEvent::Close(trace::SpanClose {
                id: self.span_id,
                attributes: vec![],
            })
            .into(),
        ))
    }
}
