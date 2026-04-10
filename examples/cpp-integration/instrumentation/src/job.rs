// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Job entity: groups tasks, root resource group.

use quent_model::{Attributes, entity};

#[derive(Debug, Attributes, serde::Serialize, serde::Deserialize)]
pub struct Submit {
    pub name: String,
    pub num_tasks: u32,
}

#[derive(Debug, Attributes, serde::Serialize, serde::Deserialize)]
pub struct Complete {
    pub metadata: quent_attributes::CustomAttributes,
}

entity! {
    Job: ResourceGroup<Root = true> {
        declaration: submit,
        events: {
            submit: Submit,
            complete: Complete,
        },
    }
}
