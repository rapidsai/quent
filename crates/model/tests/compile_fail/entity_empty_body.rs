// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// entity! with empty body and no resource group annotation should fail.
quent_model::entity! {
    Other {}
}

fn main() {}
