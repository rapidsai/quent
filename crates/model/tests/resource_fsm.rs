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
    resource(capacity = MemOperating),
    entry -> MemInitializing,
    MemInitializing -> MemOperating,
    MemOperating -> MemFinalizing,
    MemFinalizing -> exit,
)]
pub struct TestMemory;

// A unit resource (processor-like)

#[quent_model::state]
pub struct ProcInitializing;

#[quent_model::state]
pub struct ProcOperating;

#[quent_model::state]
pub struct ProcFinalizing;

#[quent_model::fsm(
    resource(capacity = ProcOperating),
    entry -> ProcInitializing,
    ProcInitializing -> ProcOperating,
    ProcOperating -> ProcFinalizing,
    ProcFinalizing -> exit,
)]
pub struct TestProcessor;

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
