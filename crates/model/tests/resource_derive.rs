// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for the `resource!` macro.

use quent_model::{Capacity, HasEventType, ModelBuilder, ModelComponent, Resource};
use uuid::Uuid;

// Fixed-bounds resource with capacity
quent_model::resource! {
    TestMem {
        capacity: { bytes: u64 },
    }
}

// Unit resource (no capacity)
quent_model::resource! { TestProc }

// Resizable resource
quent_model::resource! {
    TestResizable {
        resizable: true,
        capacity: { slots: u64 },
    }
}

#[test]
fn resource_generates_operating_with_capacity() {
    let op = TestMemOperating {
        capacity_bytes: Capacity::new(1024),
    };
    assert_eq!(op.capacity_bytes.value, 1024);
}

#[test]
fn resource_generates_initializing() {
    let init = TestMemInitializing {
        instance_name: "test".into(),
        parent_group_id: Uuid::nil(),
        resource_type_name: "test_mem".into(),
    };
    assert_eq!(init.instance_name, "test");
    assert_eq!(init.resource_type_name, "test_mem");
}

#[test]
fn unit_resource_generates_types() {
    let init = TestProcInitializing {
        instance_name: "cpu".into(),
        parent_group_id: Uuid::nil(),
        resource_type_name: "test_proc".into(),
    };
    assert_eq!(init.resource_type_name, "test_proc");
    let _op = TestProcOperating;
}

#[test]
fn resource_model_component() {
    let mut builder = ModelBuilder::new("test");
    TestMem::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "test_mem");
}

#[test]
fn resource_trait_impl() {
    fn assert_resource<T: Resource>() {}
    assert_resource::<TestMemResource>();
    assert_resource::<TestProcResource>();
}

#[test]
fn resource_has_event_type() {
    fn assert_has_event<T: HasEventType>() {}
    assert_has_event::<TestMem>();
    assert_has_event::<TestProc>();
}

#[test]
fn resizable_resource_model_component() {
    let mut builder = ModelBuilder::new("test");
    TestResizable::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "test_resizable");
    // Resizable has 4 states: initializing, operating, resizing, finalizing
    assert_eq!(fsm.states.len(), 4);
}
