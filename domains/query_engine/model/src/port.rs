// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Port entity: input or output of an operator.

use quent_model::{Attributes, Ref, entity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Declaration {
    /// The ID of the operator this port belongs to.
    pub operator_id: Ref<super::operator::Operator>,
    /// The name of this port.
    pub instance_name: String,
}

#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct Statistics {
    pub custom_attributes: quent_attributes::CustomAttributes,
}

entity! {
    Port: ResourceGroup {
        declaration: declaration,
        events: {
            declaration: Declaration,
            statistics: Statistics,
        },
    }
}
