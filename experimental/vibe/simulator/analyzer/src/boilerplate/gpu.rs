// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::gpu::{GpuEventStorage, GpuEvents};

use super::*;

pub(crate) type Gpu = StoredEntity<schema::Gpu, GpuEventStorage>;

impl RefTreeEntity for Gpu {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.worker_id.target)
    }
}
