// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for cross-crate model composition, Usage<T> with stdlib types,
//! Ref<T>, and full model collection.

use quent_model::prelude::*;

// --- Simulate a domain model crate (inline) ---

#[quent_model::entity]
pub struct Engine {
    pub name: String,
}

#[quent_model::entity]
pub struct Operator {
    pub plan_id: Uuid,
    pub type_name: String,
}

#[quent_model::event(entity = Operator)]
pub struct OperatorStatistics {
    pub rows_processed: u64,
}

// --- Application-specific types using stdlib resources ---

// Re-export stdlib types as application-specific aliases
type WorkerMemory = quent_stdlib::Memory;
type Thread = quent_stdlib::Processor;
type FsToMem = quent_stdlib::Channel;

// --- Application FSM using stdlib resource types ---

#[quent_model::state]
pub struct Queueing {
    pub operator_id: Ref<Operator>,
    pub instance_name: String,
}

#[quent_model::state]
pub struct Computing {
    #[quent_model::usage]
    pub thread: Usage<Thread>,
    #[quent_model::usage]
    pub memory: Usage<WorkerMemory>,
    #[quent_model::deferred]
    pub rows_processed: Option<u64>,
}

#[quent_model::state]
pub struct Sending {
    #[quent_model::usage]
    pub channel: Usage<FsToMem>,
}

#[quent_model::fsm(
    entry -> Queueing,
    Queueing -> Computing,
    Computing -> Sending,
    Sending -> Queueing,
    Computing -> exit,
)]
pub struct Task;

// --- Model composition ---

type DomainModel = Model<(Engine, Operator)>;
type AppModel = Model<(DomainModel, Task, WorkerMemory, Thread, FsToMem)>;

// --- Tests ---

#[test]
fn ref_type_safety() {
    // Ref<Operator> and Ref<Engine> are different types
    let op_ref: Ref<Operator> = Ref::new(Uuid::nil());
    let eng_ref: Ref<Engine> = Ref::new(Uuid::nil());

    // Both resolve to Uuid
    let _: Uuid = op_ref.uuid();
    let _: Uuid = eng_ref.uuid();

    // Into<Uuid> works
    let _: Uuid = op_ref.into();
}

#[test]
fn usage_with_stdlib_memory() {
    let usage: Usage<WorkerMemory> = Usage {
        resource_id: Ref::new(Uuid::nil()),
        capacity: quent_stdlib::MemoryOperating {
            capacity_bytes: 1024 * 1024,
        },
    };
    assert_eq!(usage.capacity.capacity_bytes, 1024 * 1024);
}

#[test]
fn usage_with_stdlib_processor() {
    let _usage: Usage<Thread> = Usage {
        resource_id: Ref::new(Uuid::nil()),
        capacity: quent_stdlib::ProcessorOperating,
    };
}

#[test]
fn usage_with_stdlib_channel() {
    let _usage: Usage<FsToMem> = Usage {
        resource_id: Ref::new(Uuid::nil()),
        capacity: quent_stdlib::ChannelOperating {
            capacity_bytes: Some(4096),
            source_id: Uuid::nil(),
            target_id: Uuid::nil(),
        },
    };
}

#[test]
fn state_with_usage_fields() {
    let computing_def = Computing::state_def();
    assert_eq!(computing_def.name, "computing");
    assert_eq!(computing_def.usages.len(), 2);
    assert_eq!(computing_def.usages[0].field_name, "thread");
    assert_eq!(computing_def.usages[1].field_name, "memory");
    assert_eq!(computing_def.deferred_attributes.len(), 1);
    assert_eq!(computing_def.deferred_attributes[0].name, "rows_processed");
}

#[test]
fn state_with_ref_field() {
    let queueing_def = Queueing::state_def();
    assert_eq!(queueing_def.name, "queueing");
    assert_eq!(queueing_def.attributes.len(), 2);
    assert_eq!(queueing_def.attributes[0].name, "operator_id");
    assert_eq!(queueing_def.attributes[1].name, "instance_name");
}

#[test]
fn full_model_collection() {
    let builder = AppModel::build();

    // Domain entities
    assert_eq!(builder.entities.len(), 2);
    assert!(builder.entities.iter().any(|e| e.name == "engine"));
    assert!(builder.entities.iter().any(|e| e.name == "operator"));

    // Application FSM + 3 stdlib resource FSMs
    assert_eq!(builder.fsms.len(), 4);
    assert!(builder.fsms.iter().any(|f| f.name == "task"));
    assert!(builder.fsms.iter().any(|f| f.name == "memory"));
    assert!(builder.fsms.iter().any(|f| f.name == "processor"));
    assert!(builder.fsms.iter().any(|f| f.name == "channel"));
}

#[test]
fn task_fsm_structure() {
    let mut builder = ModelBuilder::new();
    Task::collect(&mut builder);

    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "task");
    assert_eq!(fsm.states.len(), 3);
    assert_eq!(fsm.transitions.len(), 5);

    // Verify state names
    let state_names: Vec<&str> = fsm.states.iter().map(|s| s.name.as_str()).collect();
    assert!(state_names.contains(&"queueing"));
    assert!(state_names.contains(&"computing"));
    assert!(state_names.contains(&"sending"));

    // Verify transitions include entry and exit
    let has_entry = fsm.transitions.iter().any(|t| {
        t.from == quent_model::TransitionEndpoint::Entry
    });
    let has_exit = fsm.transitions.iter().any(|t| {
        t.to == quent_model::TransitionEndpoint::Exit
    });
    assert!(has_entry);
    assert!(has_exit);
}

#[test]
fn deferred_event_types() {
    // ComputingDeferred should have RowsProcessed variant
    let d = ComputingDeferred::RowsProcessed(42);
    match d {
        ComputingDeferred::RowsProcessed(v) => assert_eq!(v, 42),
    }

    // TaskDeferred wraps per-state deferred types
    let td = TaskDeferred::Computing(ComputingDeferred::RowsProcessed(100));
    match td {
        TaskDeferred::Computing(ComputingDeferred::RowsProcessed(v)) => assert_eq!(v, 100),
        _ => panic!("unexpected variant"),
    }
}

#[test]
fn fsm_event_sequence_numbers() {
    let transition: TaskEvent = FsmEvent::Transition {
        seq: 0,
        state: TaskTransition::Queueing(Queueing {
            operator_id: Ref::new(Uuid::nil()),
            instance_name: "test".into(),
        }),
    };
    assert_eq!(transition.seq(), 0);

    let deferred: TaskEvent = FsmEvent::Deferred {
        seq: 1,
        deferred: TaskDeferred::Computing(ComputingDeferred::RowsProcessed(42)),
    };
    assert_eq!(deferred.seq(), 1);
}

#[test]
fn ref_serde_roundtrip() {
    let original: Ref<Operator> = Ref::new(uuid::Uuid::from_u128(0x1234));
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Ref<Operator> = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn fsm_event_serde_roundtrip() {
    let event: TaskEvent = FsmEvent::Transition {
        seq: 5,
        state: TaskTransition::Sending(Sending {
            channel: Usage {
                resource_id: Ref::new(Uuid::nil()),
                capacity: quent_stdlib::ChannelOperating {
                    capacity_bytes: Some(1024),
                    source_id: Uuid::nil(),
                    target_id: Uuid::nil(),
                },
            },
        }),
    };

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: TaskEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.seq(), 5);
}
