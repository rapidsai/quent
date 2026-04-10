// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Task FSM: running on a thread.

quent_model::state! {
    Queued {
        attributes: {
            job_id: uuid::Uuid,
        },
    }
}

quent_model::state! {
    Running {
        usages: {
            thread: quent_stdlib::Processor,
        },
    }
}

quent_model::fsm! {
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
