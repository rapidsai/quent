// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Standard library of common resource definitions.
//!
//! - [`Memory`]: fixed-bounds memory resource with `bytes` capacity
//! - [`ResizableMemory`]: memory with resizable bounds
//! - [`Processor`]: unit resource for computation
//! - [`Channel`]: unidirectional data transfer resource with `bytes` capacity

mod channel;
mod memory;
mod processor;

pub use channel::*;
pub use memory::*;
pub use processor::*;
