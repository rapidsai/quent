// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! ThreadPool entity: resource group containing threads.

use quent_model::{entity, Attributes};

#[derive(Debug, Attributes, serde::Serialize, serde::Deserialize)]
pub struct ThreadPoolInit {
    pub num_threads: u32,
}

entity! {
    ThreadPool: ResourceGroup {
        declaration: init,
        events: {
            init: ThreadPoolInit,
        },
    }
}
