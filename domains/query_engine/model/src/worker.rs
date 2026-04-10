// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Worker entity: responsible for executing plans.

use quent_model::{Event, Ref};
use serde::{Deserialize, Serialize};

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Init {
    pub parent_engine_id: Ref<super::engine::Engine>,
    pub instance_name: String,
}

#[derive(Debug, Event, Deserialize, Serialize)]
pub struct Exit;

quent_model::entity! {
    Worker: ResourceGroup {
        declaration: init,
        events: {
            init: Init,
            exit: Exit,
        },
    }
}
