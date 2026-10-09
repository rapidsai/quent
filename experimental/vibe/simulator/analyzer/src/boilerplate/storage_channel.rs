// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::storage_channel::{StorageChannelEventStorage, StorageChannelEvents};

use super::*;

pub(crate) type StorageChannel = StoredEntity<schema::StorageChannel, StorageChannelEventStorage>;

impl StorageChannel {
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("storage_channel", [CapacityDecl::new_rate("bytes")])
    }
}

impl RefTreeEntity for StorageChannel {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.worker_id.target)
    }
}

impl Resource for StorageChannel {}
