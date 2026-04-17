// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Standard library of common resource definitions.
//!
//! - [`Memory`]: fixed-bounds memory resource with `bytes` capacity
//! - [`ResizableMemory`]: memory with resizable bounds
//! - [`Processor`]: unit resource for computation
//! - [`Channel`]: unidirectional data transfer resource with `bytes` capacity

pub mod channel;
pub mod memory;
pub mod processor;
