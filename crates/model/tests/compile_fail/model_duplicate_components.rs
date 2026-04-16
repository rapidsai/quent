// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// model! rejects duplicate component names.
quent_model::entity! {
    Root: ResourceGroup<Root = true> {}
}

quent_model::entity! {
    Thing {
        attributes: { x: u64 },
    }
}

quent_model::model! {
    App {
        root: Root,
        Thing,
        Thing,
    }
}

fn main() {}
