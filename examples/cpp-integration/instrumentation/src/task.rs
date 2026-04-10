// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM: running on a thread.

use quent_model::{fsm, state};

state! {
    Queued {
        attributes: {
            job_id: uuid::Uuid,
        },
    }
}

state! {
    Running {
        usages: {
            thread: quent_stdlib::Processor,
        },
    }
}

fsm! {
    Task {
        states: {
            queued: Queued,
            running: Running,
        },
        entry: queued,
        exit_from: { running },
        transitions: {
            queued => running,
            running => queued,
        },
    }
}
