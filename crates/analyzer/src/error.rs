// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error("importer error: {0}")]
    Importer(#[from] quent_exporter_types::ImporterError),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("invalid id: {0}")]
    InvalidId(Uuid),
    #[error("invalid type name: {0}")]
    InvalidTypeName(String),
    #[error("time error: {0}")]
    Time(#[from] quent_time::TimeError),
    #[error("value type error: {0}")]
    ValueType(String),
    #[error("broken implementation error: {0}")]
    BrokenImpl(&'static str),
    #[error("incomplete entity: {0}")]
    IncompleteEntity(String),
    #[error("incomplete fsm: {0}")]
    IncompleteFsm(String),
    #[error("attempted to convert exit transition into state")]
    FsmExitTransitionConversion,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}
