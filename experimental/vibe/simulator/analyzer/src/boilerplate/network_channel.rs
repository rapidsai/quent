// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::network_channel::{NetworkChannelEventStorage, NetworkChannelEvents};

use super::*;

pub(crate) type NetworkChannel = StoredEntity<schema::NetworkChannel, NetworkChannelEventStorage>;

impl NetworkChannel {
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("network_channel", [CapacityDecl::new_rate("bytes")])
    }
}

impl RefTreeEntity for NetworkChannel {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.network_id.target)
    }
}

impl Resource for NetworkChannel {}
