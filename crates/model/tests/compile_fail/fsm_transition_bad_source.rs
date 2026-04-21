// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// fsm! transition source must exist in states list.
quent_model::state! {
    Idle {}
}

quent_model::fsm! {
    Bad {
        states: { idle: Idle },
        entry: idle,
        exit_from: { idle },
        transitions: { ghost => idle },
    }
}

fn main() {}
