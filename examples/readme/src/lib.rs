// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! README example — verifies the code in the top-level README compiles.

use quent_attributes::CustomAttributes;
use quent_model::{
    Attributes, Capacity, EmitOnce, Entity, Event, Fsm, Ref, ResizableResource, Resource, State,
    Usage,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// A "unit" resource. Only one entity may use this at a time.
#[derive(Resource)]
pub struct Thread;

// A resource with a single, bounded capacity.
// Multiple entities may hold on to a certain amount of this resource simultaneously.
#[derive(Resource)]
pub struct Cache {
    pub slots: Capacity<u64>,
}

// A resource with a single, bounded capacity, which is resizable.
#[derive(ResizableResource)]
pub struct MemoryPool {
    pub bytes: Capacity<u64>,
}

// A resource with a potentially unbounded capacity (by setting the Option to none).
#[derive(Resource)]
pub struct Queue {
    pub depth: Capacity<Option<u64>>,
}

// Structs with key-value attributes
#[derive(Attributes, Serialize, Deserialize)]
pub struct AppDetails {
    pub version: String,          // key known at compile-time
    pub custom: CustomAttributes, // for keys known at run-time only
}

// A trivial single-event entity.
#[derive(Entity, Event, Serialize, Deserialize)]
pub struct Info {
    pub message: String,
    pub source: Option<String>,
}

// An arbitrary entity event
#[derive(Event, Serialize, Deserialize)]
pub struct Launched {
    pub size: u64,
}

#[derive(Event, Serialize, Deserialize)]
pub struct Collected {
    pub acknowleded: bool,
}

// An entity with an arbitrary number of one-shot events.
// TODO(johanpel): add EmitMultiple, which can be used to define e.g. metric streams
#[derive(Entity)]
pub struct AsyncSend {
    pub launched: EmitOnce<Launched>,
    pub collected: EmitOnce<Collected>,
}

// An entity can be marked as a Resource Group.
#[derive(Entity)]
#[resource_group]
pub struct Worker {
    // A ref to another resource group can be marked as a parent-child relation.
    // A resource or resource group can only have one parent.
    #[parent_group]
    pub cluster: Ref<Cluster>,
}

// There must be at least one root resource.
#[derive(Entity)]
#[resource_group(root)]
pub struct Cluster;

// Attributes of an FSM state
#[derive(State, Serialize, Deserialize)]
pub struct Queued {
    // Marks this field to carry the instance name of the entity:
    #[instance_name]
    pub name: String,
    pub worker_id: Ref<Worker>, // reference to another entity
    pub queue: Usage<Queue>,    // usage of a resource
}

#[derive(State, Serialize, Deserialize)]
pub struct Computing {
    pub thread: Usage<ThreadResource>,
    pub memory: Usage<MemoryPool>,
}

// An FSM
#[derive(Fsm)]
pub struct Task {
    #[entry]
    #[to(Computing)]
    pub queued: Queued,
    #[to(exit)]
    pub computing: Computing,
}

// Generates all event-related types.
quent_model::define_model! {
    App {
        root: Cluster,
        Worker,
        Thread,
        Cache,
        MemoryPool,
        Queue,
        AsyncSend,
        Task,
        Info,
    }
}

// Generates the isntrumentation API
quent_model::define_instrumentation!(App);

fn use_instrumentation_example() -> Result<(), Box<dyn std::error::Error>> {
    let context = AppContext::try_new(None, Uuid::now_v7())?;
    let cluster_id = context
        .cluster_observer()
        .cluster(Uuid::now_v7(), "example_cluster");
    let worker_id =
        context
            .worker_observer()
            .worker(Uuid::now_v7(), "worker_0", Ref::new(cluster_id));
    let pool_handle =
        context
            .memory_pool_observer()
            .initializing(Uuid::now_v7(), "task_pool", worker_id);
    // ... pool doing pool things goes here
    pool_handle.operating(1337);

    // instead of this:
    // .initializing(MemoryPoolInitializing {
    //     instance_name: "hi".to_string(),
    //     parent_group_id: cluster,
    //     resource_type_name: (),
    // });
    Ok(())
}
