// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Channel resource definition.

// A unidirectional data transfer resource.
quent_model::resource! {
    Channel {
        attributes: {
            source_id: uuid::Uuid,
            target_id: uuid::Uuid,
        },
        capacity: {
            rate,
            bytes: Option<u64>,
        },
    }
}
