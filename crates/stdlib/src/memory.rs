// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Memory resource definitions.

// A fixed-bounds memory resource.
quent_model::resource! {
    Memory {
        capacity: { bytes: Option<u64> }
    }
}

// A resizable memory resource.
quent_model::resource! {
    ResizableMemory {
        resizable: true,
        capacity: { bytes: Option<u64> }
    }
}
