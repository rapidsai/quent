// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::gpu_memory::{GpuMemoryEventStorage, GpuMemoryEvents};

use super::*;

pub(crate) type GpuMemory = StoredEntity<schema::GpuMemory, GpuMemoryEventStorage>;

impl GpuMemory {
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("gpu_memory", [CapacityDecl::new_occupancy("bytes")])
    }
}

impl RefTreeEntity for GpuMemory {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.gpu_id.target)
    }
}

impl Resource for GpuMemory {}
