// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for resource macro (FSM marked as a resource).

use quent_model::prelude::*;

// A memory-like resource FSM

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct MemInitializing;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct MemOperating {
    pub capacity_bytes: u64,
}

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct MemFinalizing;

#[derive(Fsm)]
#[resource(capacity = MemOperating)]
pub struct TestMemory {
    #[entry] #[to(MemOperating)]
    pub mem_initializing: MemInitializing,
    #[to(MemFinalizing)]
    pub mem_operating: MemOperating,
    #[to(exit)]
    pub mem_finalizing: MemFinalizing,
}

// A unit resource (processor-like)

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ProcInitializing;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ProcOperating;

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct ProcFinalizing;

#[derive(Fsm)]
#[resource(capacity = ProcOperating)]
pub struct TestProcessor {
    #[entry] #[to(ProcOperating)]
    pub proc_initializing: ProcInitializing,
    #[to(ProcFinalizing)]
    pub proc_operating: ProcOperating,
    #[to(exit)]
    pub proc_finalizing: ProcFinalizing,
}

#[test]
fn resource_trait_impl() {
    fn assert_resource<T: Resource>() {}
    assert_resource::<TestMemoryResource>();
    assert_resource::<TestProcessorResource>();
}

#[test]
fn usage_type_resolves() {
    // Usage<TestMemoryResource> should have capacity type MemOperating
    let _usage: Usage<TestMemoryResource> = Usage {
        resource_id: Ref::new(Uuid::nil()),
        capacity: MemOperating {
            capacity_bytes: 1024,
        },
    };

    // Usage<TestProcessorResource> should have capacity type ProcOperating
    let _usage: Usage<TestProcessorResource> = Usage {
        resource_id: Ref::new(Uuid::nil()),
        capacity: ProcOperating,
    };
}

#[test]
fn model_collection() {
    type TestModel = Model<(TestMemory, TestProcessor)>;
    let builder = TestModel::build();

    assert_eq!(builder.fsms.len(), 2);
}
