// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Query FSM: the top-level unit of work executed by an engine.

use quent_model::{Ref, fsm, state};

// States

state! {
    Init {
        attributes: {
            query_group_id: Ref<super::query_group::QueryGroup>,
        },
    }
}

state! { Planning {} }

state! { Executing {} }

// FSM: entry -> Init -> Planning -> Executing -> exit

fsm! {
    Query: ResourceGroup {
        states: {
            init: Init,
            planning: Planning,
            executing: Executing,
        },
        entry: init,
        exit_from: { executing },
        transitions: {
            init => planning,
            planning => executing,
        },
    }
}
