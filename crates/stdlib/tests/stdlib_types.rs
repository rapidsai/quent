// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests verifying stdlib resource types exist and are usable.

use quent_model::{Capacity, ModelBuilder, ModelComponent, Resource};
use uuid::Uuid;

#[test]
fn memory_types_exist() {
    let _init = quent_stdlib::MemoryInitializing {
        instance_name: "test".into(),
        parent_group_id: Uuid::nil(),
        resource_type_name: "memory".into(),
    };
    let _op = quent_stdlib::MemoryOperating {
        capacity_bytes: Capacity::new(Some(1024)),
    };
    let _fin = quent_stdlib::MemoryFinalizing;
}

#[test]
fn processor_types_exist() {
    let _init = quent_stdlib::ProcessorInitializing {
        instance_name: "cpu0".into(),
        parent_group_id: Uuid::nil(),
        resource_type_name: "processor".into(),
    };
    let _op = quent_stdlib::ProcessorOperating;
    let _fin = quent_stdlib::ProcessorFinalizing;
}

#[test]
fn channel_types_exist() {
    let _init = quent_stdlib::ChannelInitializing {
        instance_name: "ch0".into(),
        parent_group_id: Uuid::nil(),
        resource_type_name: "channel".into(),
        source_id: Uuid::nil(),
        target_id: Uuid::nil(),
    };
    let _op = quent_stdlib::ChannelOperating {
        capacity_bytes: Capacity::new(Some(4096)),
    };
    let _fin = quent_stdlib::ChannelFinalizing;
}

#[test]
fn memory_model_component() {
    let mut builder = ModelBuilder::new("test");
    quent_stdlib::Memory::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "memory");
}

#[test]
fn processor_model_component() {
    let mut builder = ModelBuilder::new("test");
    quent_stdlib::Processor::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "processor");
}

#[test]
fn channel_model_component() {
    let mut builder = ModelBuilder::new("test");
    quent_stdlib::Channel::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "channel");
}

#[test]
fn resizable_memory_model_component() {
    let mut builder = ModelBuilder::new("test");
    quent_stdlib::ResizableMemory::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "resizable_memory");
    // Resizable has 4 states: initializing, operating, resizing, finalizing
    assert_eq!(fsm.states.len(), 4);
}

#[test]
fn resource_markers_exist() {
    fn assert_resource<T: Resource>() {}
    assert_resource::<quent_stdlib::MemoryResource>();
    assert_resource::<quent_stdlib::ProcessorResource>();
    assert_resource::<quent_stdlib::ChannelResource>();
    assert_resource::<quent_stdlib::ResizableMemoryResource>();
}
