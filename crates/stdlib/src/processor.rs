// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Processor (unit resource) definition.

// A unit resource representing a processor (e.g., a thread).
// FSM: entry -> initializing -> operating -> finalizing -> exit
quent_model::resource! { Processor }
