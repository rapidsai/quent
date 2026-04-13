// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Integration test for the `state!` proc macro with flat-arg FSM methods.

use quent_model::{EventSender, ModelBuilder, ModelComponent, Ref, StateMetadata};
use uuid::Uuid;

// States defined with state! macro.
// Inline attributes auto-add `instance_name: String`.

quent_model::state! {
    Queued {
        attributes: {
            priority: u32,
        },
    }
}

quent_model::state! {
    Computing {
        usages: {
            thread: quent_stdlib::Processor,
            memory: quent_stdlib::Memory,
        },
    }
}

// FSM

quent_model::fsm! {
    Task {
        states: {
            queued: Queued,
            computing: Computing,
        },
        entry: queued,
        exit_from: { computing },
        transitions: {
            queued => computing,
        },
    }
}

// Tests

#[test]
fn state_macro_generates_struct() {
    let q = Queued {
        instance_name: "test".to_string(),
        priority: 1,
    };
    assert_eq!(q.instance_name, "test");
    assert_eq!(q.priority, 1);
}

#[test]
fn state_macro_generates_usage_struct() {
    let c = Computing {
        thread: Some(quent_model::Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::ProcessorOperating {},
        }),
        memory: Some(quent_model::Usage {
            resource_id: Ref::new(Uuid::nil()),
            capacity: quent_stdlib::MemoryOperating {
                capacity_bytes: quent_model::Capacity::new(Some(1024)),
            },
        }),
    };
    assert_eq!(
        c.memory.as_ref().unwrap().capacity.capacity_bytes.value,
        Some(1024)
    );
}

#[test]
fn state_metadata() {
    assert_eq!(Queued::state_name(), "queued");
    assert_eq!(Computing::state_name(), "computing");

    let queued_def = Queued::state_def();
    assert_eq!(queued_def.name, "queued");
    // instance_name + priority
    assert_eq!(queued_def.attributes.len(), 2);
    assert_eq!(queued_def.attributes[0].name, "instance_name");
    assert_eq!(queued_def.attributes[1].name, "priority");
    assert_eq!(queued_def.usages.len(), 0);

    let computing_def = Computing::state_def();
    assert_eq!(computing_def.name, "computing");
    assert_eq!(computing_def.attributes.len(), 0);
    assert_eq!(computing_def.usages.len(), 2);
    assert_eq!(computing_def.usages[0].field_name, "thread");
    assert_eq!(computing_def.usages[1].field_name, "memory");
}

#[test]
fn fsm_model_component() {
    let mut builder = ModelBuilder::new("test");
    Task::collect(&mut builder);

    assert_eq!(builder.fsms.len(), 1);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "task");
    assert_eq!(fsm.states.len(), 2);
    // entry->queued, queued->computing, computing->exit = 3 transitions
    assert_eq!(fsm.transitions.len(), 3);
}

#[test]
fn flat_args_observer_entry() {
    let tx: EventSender<TaskEvent> = EventSender::default();
    let observer = TaskObserver::new(&tx);
    let id = Uuid::nil();

    let mut handle = observer.queued(id, "my_task", 5);
    assert_eq!(handle.uuid(), id);
    handle.exit();
}

#[test]
fn flat_args_handle_transition() {
    let tx: EventSender<TaskEvent> = EventSender::default();
    let observer = TaskObserver::new(&tx);
    let id = Uuid::nil();

    let mut handle = observer.queued(id, "my_task", 5);

    handle.computing(
        Some(quent_model::usage(Ref::<quent_stdlib::Processor>::new(
            Uuid::nil(),
        ))),
        Some(quent_model::usage((
            Ref::<quent_stdlib::Memory>::new(Uuid::nil()),
            2048,
        ))),
    );
    handle.exit();
}

#[test]
fn extract_instance_name() {
    use quent_model::analyze::ExtractInstanceName;
    let q = Queued {
        instance_name: "test_task".to_string(),
        priority: 1,
    };
    assert_eq!(q.extract_instance_name(), Some("test_task"));
}

#[test]
fn extract_usages() {
    use quent_model::analyze::ExtractUsages;
    let c = Computing {
        thread: Some(quent_model::Usage {
            resource_id: Ref::new(Uuid::from_u128(1)),
            capacity: quent_stdlib::ProcessorOperating {},
        }),
        memory: Some(quent_model::Usage {
            resource_id: Ref::new(Uuid::from_u128(2)),
            capacity: quent_stdlib::MemoryOperating {
                capacity_bytes: quent_model::Capacity::new(Some(4096)),
            },
        }),
    };
    let usages = c.extract_usages();
    assert_eq!(usages.len(), 2);
    assert_eq!(usages[0].resource_id, Uuid::from_u128(1));
    assert_eq!(usages[1].resource_id, Uuid::from_u128(2));
}

#[test]
fn extract_usages_skips_none() {
    use quent_model::analyze::ExtractUsages;
    let c = Computing {
        thread: None,
        memory: Some(quent_model::Usage {
            resource_id: Ref::new(Uuid::from_u128(2)),
            capacity: quent_stdlib::MemoryOperating {
                capacity_bytes: quent_model::Capacity::new(Some(4096)),
            },
        }),
    };
    let usages = c.extract_usages();
    assert_eq!(usages.len(), 1);
    assert_eq!(usages[0].resource_id, Uuid::from_u128(2));
}
