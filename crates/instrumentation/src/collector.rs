// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Collector integration for generated instrumentation models.

use crate::{Context, InstrumentedModel};

#[doc(hidden)]
pub use quent_collector_client::{CollectorSink, deserialize_event, serialize_event};

/// Routes serialized collector events into a generated model context.
#[doc(hidden)]
pub trait CollectorRouter: quent_events::Model + InstrumentedModel + Sized {
    fn dispatch(
        context: &Context<Self>,
        entity: &str,
        event: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>>;
}

impl<M: CollectorRouter> CollectorSink for Context<M> {
    fn ingest(&self, entity: &str, event: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        M::dispatch(self, entity, event)
    }
}
