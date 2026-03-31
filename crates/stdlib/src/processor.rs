// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Processor (unit resource) FSM definition.

#[allow(unused_imports)]
use quent_model::prelude::*;

#[quent_model::state]
pub struct ProcessorInitializing;

#[quent_model::state]
pub struct ProcessorOperating;

#[quent_model::state]
pub struct ProcessorFinalizing;

/// A unit resource representing a processor (e.g., a thread).
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// A unit resource has no capacity fields — `Usage<Processor>` only carries
/// the `resource_id`.
#[quent_model::fsm(
    resource(capacity = ProcessorOperating),
    entry -> ProcessorInitializing,
    ProcessorInitializing -> ProcessorOperating,
    ProcessorOperating -> ProcessorFinalizing,
    ProcessorFinalizing -> exit,
)]
pub struct Processor;
