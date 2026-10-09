// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::storage::{StorageEventStorage, StorageEvents};

use super::*;

pub(crate) type Storage = StoredEntity<schema::Storage, StorageEventStorage>;

impl Storage {
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("storage", [CapacityDecl::new_occupancy("bytes")])
    }
}

impl RefTreeEntity for Storage {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.worker_id.target)
    }
}

impl Resource for Storage {}
