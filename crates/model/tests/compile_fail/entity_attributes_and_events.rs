// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// entity! cannot have both attributes and events on a non-resource-group.
quent_model::entity! {
    Bad {
        attributes: {
            x: u64,
        },
        events: {
            a: (),
        },
    }
}

fn main() {}
