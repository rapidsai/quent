// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Standard library of common resource FSM definitions.
//!
//! These are predefined FSMs matching the spec's common entity types:
//! - [`Memory`]: bounded resource with `bytes` occupancy capacity
//! - [`DynamicMemory`]: memory with resizable bounds
//! - [`Processor`]: unit resource for computation
//! - [`Channel`]: unidirectional data transfer resource with `bytes` rate capacity

mod channel;
mod memory;
mod processor;

pub use channel::*;
pub use memory::*;
pub use processor::*;
