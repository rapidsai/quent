// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! README example — verifies the code in the top-level README compiles.

use quent_attributes::{Attribute, CustomAttributes};
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
    pub capacity_slots: Capacity<u64>,
}

// A resource with a single, bounded capacity, which is resizable.
#[derive(ResizableResource)]
pub struct MemoryPool {
    pub capacity_bytes: Capacity<u64>,
}

// A resource with a potentially unbounded capacity (by setting the Option to none).
#[derive(Resource)]
pub struct Queue {
    pub capacity_depth: Capacity<Option<u64>>,
}

// A trivial single-event entity.
#[derive(Entity, Event, Serialize, Deserialize)]
pub struct Info {
    pub message: String,
    pub source: Option<String>,
}

// Events for a multi-event entity
#[derive(Event, Serialize, Deserialize)]
pub struct Checksum {
    pub algorithm: String,
    pub value: String,
}

#[derive(Event, Serialize, Deserialize)]
pub struct Decompressed {
    pub algorithm: String,
    pub ratio: f64,
}

// A multi-event entity.
// Each event can arrive independently, in any order.
#[derive(Entity)]
pub struct FileStats {
    pub checksum: EmitOnce<Checksum>,
    pub decompressed: EmitOnce<Decompressed>,
}

// Structs with key-value attributes
#[derive(Attributes, Serialize, Deserialize)]
pub struct Details {
    pub version: String,          // key known at compile-time
    pub custom: CustomAttributes, // for keys known at run-time only
}

// An entity can be marked as a Resource Group.
#[derive(Entity)]
#[resource_group]
pub struct Worker {
    // A ref to another resource group can be marked as a parent-child relation.
    // A resource or resource group can only have one parent.
    #[parent_group]
    pub cluster: Ref<Cluster>,
    // Nested attributes
    pub details: Details,
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
        FileStats,
        Task,
        Info,
    }
}

// Generates the isntrumentation API
quent_model::define_instrumentation!(App);

fn use_instrumentation_example() -> Result<(), Box<dyn std::error::Error>> {
    let context = AppContext::try_new(None, Uuid::now_v7())?;
    // Spawn a cluster
    let cluster_id = context
        .cluster_observer()
        .cluster(Uuid::now_v7(), "example_cluster");

    // Spawn a worker.
    let worker_id = context.worker_observer().worker(
        Uuid::now_v7(),
        "worker_0",
        Ref::new(cluster_id),
        Details {
            version: "42.1.2".to_string(),
            custom: vec![Attribute::u64("threads", 256)].into(),
        },
    );

    // Construct a queue.
    let mut queue_handle =
        context
            .queue_observer()
            .initializing(Uuid::now_v7(), "my_queue", worker_id);
    // ... queue getting spawned goes here
    queue_handle.operating(None);

    // Construct a memory pool.
    let mut mem_pool_handle =
        context
            .memory_pool_observer()
            .initializing(Uuid::now_v7(), "my_memory_pool", worker_id);
    // ... pool doing pool things goes here
    mem_pool_handle.operating(1337);

    // Spawn a thread.
    let mut thread_handle =
        context
            .thread_observer()
            .initializing(Uuid::now_v7(), "my_thread", worker_id);
    // ... thread getting spawned and handle moved into it goes here
    thread_handle.operating();

    // Single event entity
    context.info_observer().info(
        Uuid::now_v7(),
        "ready to operate".to_string(),
        Some(std::file!().to_string()),
    );

    // Multi-event entities can emit in any order from an entity handle.
    let mut file_stats_handle = context.file_stats_observer().create(Uuid::now_v7());
    // Pretend to calculate a checksum and decompress in parallel.
    file_stats_handle.checksum(Checksum {
        algorithm: "sha256".to_string(),
        value: "abc123def456".to_string(),
    });
    // Calculate other stuff
    file_stats_handle.decompressed(Decompressed {
        algorithm: "snappy".to_string(),
        ratio: 0.4,
    });

    // Queue a task. The entry transition returns an FSM handle.
    let task = context.task_observer().queued(Uuid::now_v7(), "my_task_31415", worker_id, /*use_queue:*/ queue_handle.into(), /*use_queue_slots:*/ 1 });

    Ok(())
}
