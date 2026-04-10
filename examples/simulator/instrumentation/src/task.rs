// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM for the simulator.
//!
//! A Task represents a unit of work executing on behalf of an operator.
//! It transitions through states like queueing, computing, allocating,
//! loading, spilling, and sending.

use quent_model::{fsm, state};
use uuid::Uuid;

// States

state! {
    Queueing {
        attributes: {
            operator_id: Uuid,
        },
    }
}

state! {
    Computing {
        usages: {
            use_thread: quent_stdlib::Processor,
            use_memory: quent_stdlib::Memory,
        },
    }
}

state! {
    Allocating {
        usages: {
            use_thread: quent_stdlib::Processor,
        },
    }
}

state! {
    Loading {
        usages: {
            use_thread: quent_stdlib::Processor,
            use_fs_to_mem: quent_stdlib::Channel,
            use_memory: quent_stdlib::Memory,
        },
    }
}

state! {
    Spilling {
        usages: {
            use_thread: quent_stdlib::Processor,
            use_mem_to_fs: quent_stdlib::Channel,
        },
    }
}

state! {
    Sending {
        usages: {
            use_thread: quent_stdlib::Processor,
            use_link: quent_stdlib::Channel,
        },
    }
}

// FSM

fsm! {
    Task {
        states: {
            queueing: Queueing,
            computing: Computing,
            allocating: Allocating,
            loading: Loading,
            spilling: Spilling,
            sending: Sending,
        },
        entry: queueing,
        exit_from: { computing },
        transitions: {
            queueing => allocating,
            allocating => computing,
            allocating => loading,
            loading => computing,
            computing => sending,
            computing => spilling,
            spilling => allocating,
            sending => queueing,
        },
    }
}
