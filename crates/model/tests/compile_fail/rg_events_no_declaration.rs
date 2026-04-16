// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Resource group with events requires `declaration:` keyword.
#[derive(quent_model::Attributes, serde::Serialize, serde::Deserialize)]
pub struct MyEvent {}

quent_model::entity! {
    Bad: ResourceGroup {
        events: {
            a: MyEvent,
        },
    }
}

fn main() {}
