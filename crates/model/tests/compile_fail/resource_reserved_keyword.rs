// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// `rate` and `occupancy` are reserved keywords in capacity blocks.
quent_model::resource! {
    Bad {
        capacity: { slots: u64, rate: u64 },
    }
}

fn main() {}
