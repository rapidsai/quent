// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Processor (unit resource) FSM definition.

#[allow(unused_imports)]
use quent_model::prelude::*;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ProcessorInitializing;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ProcessorOperating;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ProcessorFinalizing;

/// A unit resource representing a processor (e.g., a thread).
///
/// FSM: `entry -> initializing -> operating -> finalizing -> exit`
///
/// A unit resource has no capacity fields -- `Usage<Processor>` only carries
/// the `resource_id`.
#[derive(Fsm)]
#[resource(capacity = ProcessorOperating)]
pub struct Processor {
    #[entry] #[to(ProcessorOperating)]
    processor_initializing: ProcessorInitializing,
    #[to(ProcessorFinalizing)]
    processor_operating: ProcessorOperating,
    #[to(exit)]
    processor_finalizing: ProcessorFinalizing,
}
