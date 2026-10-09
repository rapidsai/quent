// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::network::{NetworkEventStorage, NetworkEvents};

use super::*;

pub(crate) type Network = StoredEntity<schema::Network, NetworkEventStorage>;

impl RefTreeEntity for Network {
    fn parent_id(&self) -> Option<Uuid> {
        Some(self.declaration().data.engine_id.target)
    }
}
