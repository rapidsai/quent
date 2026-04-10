// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Memory resource definitions.

// A fixed-bounds memory resource.
// FSM: entry -> initializing -> operating -> finalizing -> exit
quent_model::resource! {
    Memory {
        capacity: { bytes: Option<u64> }
    }
}

// A resizable memory resource.
// FSM: entry -> initializing -> operating <-> resizing -> finalizing -> exit
quent_model::resource! {
    ResizableMemory {
        resizable: true,
        capacity: { bytes: Option<u64> }
    }
}
