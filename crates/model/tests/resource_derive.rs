// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for `#[derive(Resource)]` and `#[derive(ResizableResource)]`.

use quent_model::prelude::*;
use quent_model::{HasEventType, ModelBuilder, ModelComponent};

// Fixed-bounds resource with capacity

#[derive(Resource)]
pub struct TestMem {
    pub bytes: Capacity<u64>,
}

// Unit resource (no capacity)

#[derive(Resource)]
pub struct TestProc;

#[test]
fn resource_generates_operating_with_capacity() {
    let op = TestMemOperating { bytes: Capacity::new(1024) };
    assert_eq!(op.bytes.value, 1024);
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
    // Unit operating state exists as a unit struct
    let _op = TestProcOperating;
}

#[test]
fn resource_model_component() {
    let mut builder = ModelBuilder::new();
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

// Resizable resource

#[derive(ResizableResource)]
pub struct TestResizable {
    pub slots: Capacity<u64>,
}

#[test]
fn resizable_resource_model_component() {
    let mut builder = ModelBuilder::new();
    TestResizable::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "test_resizable");
    // Resizable has 4 states: initializing, operating, resizing, finalizing
    assert_eq!(fsm.states.len(), 4);
}
