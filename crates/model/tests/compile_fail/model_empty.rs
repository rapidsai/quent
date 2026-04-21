// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// model! with empty body should fail — requires root resource group.
quent_model::model! {
    App {}
}

fn main() {}
