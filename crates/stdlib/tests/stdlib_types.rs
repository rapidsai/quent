// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests verifying stdlib resource types exist and are usable.

use quent_model::{Capacity, ModelBuilder, ModelComponent, Resource};
use uuid::Uuid;

#[test]
fn memory_types_exist() {
    let _init = quent_stdlib::memory::MemoryInitializing {
        instance_name: "test".into(),
        parent_group_id: Uuid::nil(),
        resource_type_name: "memory".into(),
    };
    let _op = quent_stdlib::memory::MemoryOperating {
        capacity_bytes: Capacity::new(Some(1024)),
    };
    let _fin = quent_stdlib::memory::MemoryFinalizing;
}

#[test]
fn processor_types_exist() {
    let _init = quent_stdlib::processor::ProcessorInitializing {
        instance_name: "cpu0".into(),
        parent_group_id: Uuid::nil(),
        resource_type_name: "processor".into(),
    };
    let _op = quent_stdlib::processor::ProcessorOperating;
    let _fin = quent_stdlib::processor::ProcessorFinalizing;
}

#[test]
fn channel_types_exist() {
    let _init = quent_stdlib::channel::ChannelInitializing {
        instance_name: "ch0".into(),
        parent_group_id: Uuid::nil(),
        resource_type_name: "channel".into(),
        source_id: Uuid::nil(),
        target_id: Uuid::nil(),
    };
    let _op = quent_stdlib::channel::ChannelOperating {
        capacity_bytes: Capacity::new(Some(4096)),
    };
    let _fin = quent_stdlib::channel::ChannelFinalizing;
}

#[test]
fn memory_model_component() {
    let mut builder = ModelBuilder::new("test");
    quent_stdlib::memory::Memory::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "memory");
}

#[test]
fn processor_model_component() {
    let mut builder = ModelBuilder::new("test");
    quent_stdlib::processor::Processor::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "processor");
}

#[test]
fn channel_model_component() {
    let mut builder = ModelBuilder::new("test");
    quent_stdlib::channel::Channel::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    assert_eq!(builder.fsms[0].name, "channel");
}

#[test]
fn resizable_memory_model_component() {
    let mut builder = ModelBuilder::new("test");
    quent_stdlib::memory::ResizableMemory::collect(&mut builder);
    assert_eq!(builder.fsms.len(), 1);
    let fsm = &builder.fsms[0];
    assert_eq!(fsm.name, "resizable_memory");
    // Resizable has 4 states: initializing, operating, resizing, finalizing
    assert_eq!(fsm.states.len(), 4);
}

#[test]
fn resource_markers_exist() {
    fn assert_resource<T: Resource>() {}
    assert_resource::<quent_stdlib::memory::MemoryResource>();
    assert_resource::<quent_stdlib::processor::ProcessorResource>();
    assert_resource::<quent_stdlib::channel::ChannelResource>();
    assert_resource::<quent_stdlib::memory::ResizableMemoryResource>();
}
