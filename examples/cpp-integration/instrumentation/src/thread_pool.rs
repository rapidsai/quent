// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! ThreadPool entity: resource group containing threads.

use quent_model::Event;

#[derive(Debug, Event, serde::Serialize, serde::Deserialize)]
pub struct ThreadPoolInit {
    pub num_threads: u32,
}

quent_model::entity! {
    ThreadPool {
        resource_group: child,
        events: {
            init: ThreadPoolInit,
        },
    }
}
