// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! ThreadPool entity: resource group containing threads.

use quent_model::{EmitOnce, Entity, Event};

#[derive(Debug, Event, serde::Serialize, serde::Deserialize)]
pub struct ThreadPoolInit {
    pub num_threads: u32,
}

#[derive(Entity)]
#[resource_group]
pub struct ThreadPool {
    pub init: EmitOnce<ThreadPoolInit>,
}
