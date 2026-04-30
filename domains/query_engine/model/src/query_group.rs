// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! QueryGroup entity: encapsulates a set of queries.

use quent_model::{Attributes, entity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Attributes, Serialize, Deserialize)]
pub struct Declaration {
    /// The name of this instance of a query group.
    pub instance_name: String,
    /// The id of the engine this query group is executed on.
    pub engine_id: Uuid,
}

entity! {
    QueryGroup: ResourceGroup {
        declaration: declaration,
        events: {
            declaration: Declaration,
        },
    }
}
