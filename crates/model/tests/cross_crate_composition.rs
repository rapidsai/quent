// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for cross-crate model composition, Usage<T> with stdlib types,
//! Ref<T>, and full model collection.

use quent_model::{
    Capacity, Entity, Fsm, FsmEvent, Model, ModelBuilder, ModelComponent, Ref, State,
    StateMetadata, TransitionEndpoint, Usage,
};
use uuid::Uuid;

// Simulate a domain model crate (inline)

#[derive(Entity)]
pub struct Engine;

#[derive(Entity)]
pub struct Operator;

// Application-specific types using stdlib resources

// Use the resource marker types for Usage<T>
type WorkerMemory = quent_stdlib::MemoryResource;
type Thread = quent_stdlib::ProcessorResource;
type FsToMem = quent_stdlib::ChannelResource;

// Application FSM using stdlib resource types

#[derive(Debug, Clone, State)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Queueing {
    pub operator_id: Ref<Operator>,
    pub instance_name: String,
}

#[derive(Debug, Clone, State)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Computing {
    pub thread: Usage<Thread>,
    pub memory: Usage<WorkerMemory>,
    pub rows_processed: Option<u64>,
}

#[derive(Debug, Clone, State)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sending {
    pub channel: Usage<FsToMem>,
}

#[derive(Fsm)]
pub struct Task {
    #[entry]
    #[to(Computing)]
    pub queueing: Queueing,
    #[to(Sending, exit)]
    pub computing: Computing,
    #[to(Queueing)]
    pub sending: Sending,
}

// Model composition

type DomainModel = Model<(Engine, Operator)>;
// Use the metadata marker types in the model, not the resource marker types
type AppModel = Model<(
    DomainModel,
    Task,
    quent_stdlib::Memory,
    quent_stdlib::Processor,
    quent_stdlib::Channel,
)>;

// Tests

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
            capacity_bytes: Capacity::new(Some(1024 * 1024)),
        },
    };
    assert_eq!(usage.capacity.capacity_bytes.value, Some(1024 * 1024));
}

#[test]
fn usage_with_stdlib_processor() {
    let _usage: Usage<Thread> = Usage {
        resource_id: Ref::new(Uuid::nil()),
        // ProcessorOperating is a unit struct (no capacity fields)
        capacity: quent_stdlib::ProcessorOperating {},
    };
}

#[test]
fn usage_with_stdlib_channel() {
    let _usage: Usage<FsToMem> = Usage {
        resource_id: Ref::new(Uuid::nil()),
        capacity: quent_stdlib::ChannelOperating {
            capacity_bytes: Capacity::new(Some(4096)),
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
    let has_entry = fsm
        .transitions
        .iter()
        .any(|t| t.from == TransitionEndpoint::Entry);
    let has_exit = fsm
        .transitions
        .iter()
        .any(|t| t.to == TransitionEndpoint::Exit);
    assert!(has_entry);
    assert!(has_exit);
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
}

#[cfg(feature = "serde")]
#[test]
fn ref_serde_roundtrip() {
    let original: Ref<Operator> = Ref::new(uuid::Uuid::from_u128(0x1234));
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Ref<Operator> = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[cfg(feature = "serde")]
#[test]
fn fsm_event_serde_roundtrip() {
    let event: TaskEvent = FsmEvent::Transition {
        seq: 5,
        state: TaskTransition::Sending(Sending {
            channel: Usage {
                resource_id: Ref::new(Uuid::nil()),
                capacity: quent_stdlib::ChannelOperating {
                    capacity_bytes: Capacity::new(Some(1024)),
                },
            },
        }),
    };

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: TaskEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.seq(), 5);
}
