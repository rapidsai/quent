// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for resource macro (FSM marked as a resource).

use quent_model::prelude::*;

// A memory-like resource FSM

#[quent_model::state]
pub struct MemInitializing;

#[quent_model::state]
pub struct MemOperating {
    pub capacity_bytes: u64,
}

#[quent_model::state]
pub struct MemFinalizing;

#[quent_model::fsm(
    entry -> MemInitializing,
    MemInitializing -> MemOperating,
    MemOperating -> MemFinalizing,
    MemFinalizing -> exit,
)]
#[quent_model::resource(capacity = MemOperating)]
pub struct TestMemory;

// A unit resource (processor-like)

#[quent_model::state]
pub struct ProcInitializing;

#[quent_model::state]
pub struct ProcOperating;

#[quent_model::state]
pub struct ProcFinalizing;

#[quent_model::fsm(
    entry -> ProcInitializing,
    ProcInitializing -> ProcOperating,
    ProcOperating -> ProcFinalizing,
    ProcFinalizing -> exit,
)]
#[quent_model::resource(capacity = ProcOperating)]
pub struct TestProcessor;

#[test]
fn resource_trait_impl() {
    fn assert_resource<T: Resource>() {}
    assert_resource::<TestMemory>();
    assert_resource::<TestProcessor>();
}

#[test]
fn usage_type_resolves() {
    // Usage<TestMemory> should have capacity type MemOperating
    let _usage: Usage<TestMemory> = Usage {
        resource_id: Ref::new(Uuid::nil()),
        capacity: MemOperating {
            capacity_bytes: 1024,
        },
    };

    // Usage<TestProcessor> should have capacity type ProcOperating (unit struct)
    let _usage: Usage<TestProcessor> = Usage {
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
