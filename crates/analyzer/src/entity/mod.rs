// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Entity analysis interfaces and storage implementations.

use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

use crate::{AnalyzerResult, Span};

pub mod native;

/// Trait for analysis-time types that represent an entity.
pub trait Entity {
    /// Return the universally unique identifier.
    fn id(&self) -> Uuid;
    /// Return the type name.
    fn type_name(&self) -> &str;
    /// Return the earliest observed event timestamp.
    fn earliest_timestamp(&self) -> TimeUnixNanoSec;
    /// Return the latest observed event timestamp.
    fn latest_timestamp(&self) -> TimeUnixNanoSec;
}

impl<E: Entity> Span for E {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        Ok(SpanUnixNanoSec::try_new(
            self.earliest_timestamp(),
            self.latest_timestamp(),
        )?)
    }
}
