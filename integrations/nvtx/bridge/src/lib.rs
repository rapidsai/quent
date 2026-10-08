// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Forwards owned NVTX records to a Quent observer through a capture worker.

mod capture;
mod convert;

pub use capture::{Capture, CaptureError};
pub use convert::convert;
