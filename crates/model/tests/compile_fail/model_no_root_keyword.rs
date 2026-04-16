// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// model! first entry must be `root: <Type>`.
quent_model::entity! {
    Cluster: ResourceGroup<Root = true> {}
}

quent_model::model! {
    App {
        Cluster,
    }
}

fn main() {}
