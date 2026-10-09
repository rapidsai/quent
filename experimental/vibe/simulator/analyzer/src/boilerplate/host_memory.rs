// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::host_memory::{HostMemoryEventStorage, HostMemoryEvents};

use super::*;

pub(crate) type HostMemory = StoredEntity<schema::HostMemory, HostMemoryEventStorage>;

impl HostMemory {
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("host_memory", [CapacityDecl::new_occupancy("bytes")])
    }
}

impl RefTreeEntity for HostMemory {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.worker_id.target)
    }
}

impl Resource for HostMemory {}
