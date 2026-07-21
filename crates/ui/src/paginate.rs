// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Pagination parameters shared across list endpoints.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Parameters for paginated lists.
#[derive(TS, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageParams {
    /// The maximum size of a page.
    pub max: u32,
    /// The zero-based index of the requested page.
    pub page: u32,
}
