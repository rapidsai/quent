// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::pcie_channel::{PcieChannelEventStorage, PcieChannelEvents};

use super::*;

pub(crate) type PcieChannel = StoredEntity<schema::PcieChannel, PcieChannelEventStorage>;

impl PcieChannel {
    pub(crate) fn resource_type_decl() -> ResourceTypeDecl {
        ResourceTypeDecl::new("pcie_channel", [CapacityDecl::new_rate("bytes")])
    }
}

impl RefTreeEntity for PcieChannel {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.gpu_id.target)
    }
}

impl Resource for PcieChannel {}
