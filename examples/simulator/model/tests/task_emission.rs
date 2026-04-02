// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Integration test: verify that the Task FSM handle emits events correctly
//! through an EventSender.

use quent_model::prelude::*;
use quent_simulator_model::task::*;

// Define a top-level event type that wraps TaskEvent.
#[derive(Debug, serde::Serialize)]
enum TestEvent {
    Task(TaskEvent),
}

impl From<TaskEvent> for TestEvent {
    fn from(e: TaskEvent) -> Self {
        TestEvent::Task(e)
    }
}

#[test]
fn task_handle_lifecycle() {
    // Create a noop context (no exporter) — events are sent but discarded.
    // This tests that the handle API compiles and runs without panics.
    let ctx = quent_instrumentation::Context::<TestEvent>::try_new(None, Uuid::now_v7()).unwrap();
    let tx = ctx.events_sender();

    // Create task — emits entry -> Queueing (seq 0)
    let mut task = TaskHandle::queueing(
        &tx,
        Queueing {
            operator_id: Uuid::nil(),
            instance_name: "test-task".into(),
        },
    );

    let task_id = task.uuid();
    assert_ne!(task_id, Uuid::nil());

    // Transition to Allocating (seq 1)
    task.transition(Allocating {
        use_thread: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ProcessorOperating {},
        },
    });

    // Transition to Computing (seq 2)
    task.transition(Computing {
        use_thread: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ProcessorOperating {},
        },
        use_memory: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::MemoryOperating {
                capacity_bytes: Capacity::new(Some(4096)),
            },
        },
    });

    // Exit (seq 3)
    task.exit();
}

#[test]
fn task_handle_auto_exit_on_drop() {
    let ctx = quent_instrumentation::Context::<TestEvent>::try_new(None, Uuid::now_v7()).unwrap();
    let tx = ctx.events_sender();

    // Create and immediately drop — should not panic, auto-exits.
    let task = TaskHandle::queueing(
        &tx,
        Queueing {
            operator_id: Uuid::nil(),
            instance_name: "dropped".into(),
        },
    );
    drop(task);
}

#[test]
fn task_transition_types() {
    // Verify that each state type can be passed to transition()
    let ctx = quent_instrumentation::Context::<TestEvent>::try_new(None, Uuid::now_v7()).unwrap();
    let tx = ctx.events_sender();

    let mut task = TaskHandle::queueing(
        &tx,
        Queueing {
            operator_id: Uuid::nil(),
            instance_name: "type-test".into(),
        },
    );

    // Allocating
    task.transition(Allocating {
        use_thread: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ProcessorOperating {},
        },
    });

    // Loading
    task.transition(Loading {
        use_thread: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ProcessorOperating {},
        },
        use_fs_to_mem: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ChannelOperating {
                capacity_bytes: Capacity::new(Some(1024)),
            },
        },
        use_memory: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::MemoryOperating {
                capacity_bytes: Capacity::new(Some(8192)),
            },
        },
    });

    // Computing
    task.transition(Computing {
        use_thread: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ProcessorOperating {},
        },
        use_memory: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::MemoryOperating {
                capacity_bytes: Capacity::new(Some(4096)),
            },
        },
    });

    // Sending
    task.transition(Sending {
        use_thread: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ProcessorOperating {},
        },
        use_link: Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ChannelOperating {
                capacity_bytes: Capacity::new(None),
            },
        },
    });

    // Back to Queueing
    task.transition(Queueing {
        operator_id: Uuid::nil(),
        instance_name: "cycle".into(),
    });

    task.exit();
}
