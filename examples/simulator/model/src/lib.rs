// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Simulator application model.
//!
//! Defines FSMs, resources, and entities for the simulator example using
//! quent-model proc macros. Generated types replace the hand-written event
//! types in `quent-simulator-events` and observers in
//! `quent-simulator-instrumentation`.

#[allow(unused_imports)]
use quent_model::prelude::*;

pub mod task;
