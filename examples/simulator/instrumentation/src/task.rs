// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM for the simulator.
//!
//! A Task represents a unit of work executing on behalf of an operator.
//! It transitions through states like queueing, computing, allocating,
//! loading, spilling, and sending.

use uuid::Uuid;

// States

quent_model::state! {
    Queueing {
        attributes: {
            operator_id: Uuid,
        },
    }
}

quent_model::state! {
    Computing {
        usages: {
            use_thread: quent_stdlib::Processor,
            use_memory: quent_stdlib::Memory,
        },
    }
}

quent_model::state! {
    Allocating {
        usages: {
            use_thread: quent_stdlib::Processor,
        },
    }
}

quent_model::state! {
    Loading {
        usages: {
            use_thread: quent_stdlib::Processor,
            use_fs_to_mem: quent_stdlib::Channel,
            use_memory: quent_stdlib::Memory,
        },
    }
}

quent_model::state! {
    Spilling {
        usages: {
            use_thread: quent_stdlib::Processor,
            use_mem_to_fs: quent_stdlib::Channel,
        },
    }
}

quent_model::state! {
    Sending {
        usages: {
            use_thread: quent_stdlib::Processor,
            use_link: quent_stdlib::Channel,
        },
    }
}

// FSM

quent_model::fsm! {
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
