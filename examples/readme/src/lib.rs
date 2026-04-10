// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! README example — verifies the code in the top-level README compiles.

use quent_attributes::CustomAttributes;
use quent_model::{Attributes, Event, Ref, entity, fsm, instrumentation, model, resource, state};
use serde::{Deserialize, Serialize};

// A "unit" resource. Only one entity may use this at a time.
resource! { Thread }

// A resource with a single, bounded capacity.
resource! {
    Cache {
        capacity: { slots: u64 },
    }
}

// A resource with a single, bounded capacity, which is resizable.
resource! {
    MemoryPool {
        resizable: true,
        capacity: { bytes: u64 },
    }
}

// A resource with a potentially unbounded capacity (by setting the Option to none).
resource! {
    Queue {
        capacity: { entries: Option<u64> },
    }
}

// A trivial single-event entity.
entity! {
    Info {
        attributes: {
            message: String,
            source: Option<String>,
        },
    }
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
entity! {
    FileStats {
        events: {
            checksum: Checksum,
            decompressed: Decompressed,
        },
    }
}

// Structs with key-value attributes
#[derive(Attributes, Serialize, Deserialize)]
pub struct Details {
    pub version: String,          // key known at compile-time
    pub custom: CustomAttributes, // for keys known at run-time only
}

// An entity can be marked as a Resource Group.
// If it can only have one type of parent T, this can be
// set using Parent = T
entity! {
    Worker: ResourceGroup<Parent = Cluster> {
        attributes: {
            details: Details,
        },
    }
}

// There must be at least one root resource group.
entity! {
    Cluster: ResourceGroup<Root = true> {}
}

// FSM states separate attributes from resource usages.
state! {
    Queued {
        attributes: {
            worker: Ref<Worker>,
        },
        usages: {
            queue: Queue,
        },
    }
}

state! {
    Computing {
        usages: {
            thread: Thread,
            memory: MemoryPool,
        },
    }
}

// An FSM
fsm! {
    Task {
        states: {
            queued: Queued,
            computing: Computing,
        },
        entry: queued,
        exit_from: { computing },
        transitions: {
            queued => computing,
            computing => computing,
        },
    }
}

// Generates all event-related types.
model! {
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

// Generates the instrumentation API
instrumentation!(App);

#[test]
fn use_instrumentation_example() -> Result<(), Box<dyn std::error::Error>> {
    use quent_attributes::Attribute;
    use quent_model::usage;
    use uuid::Uuid;

    let context = AppContext::try_new(None, Uuid::now_v7())?;
    // Spawn a cluster
    let cluster = context
        .cluster_observer()
        .cluster(Uuid::now_v7(), "example_cluster");

    // Spawn a worker.
    let worker = context.worker_observer().worker(
        Uuid::now_v7(),
        "worker_0",
        Ref::new(cluster),
        Details {
            version: "42.1.2".to_string(),
            custom: vec![Attribute::u64("threads", 256)].into(),
        },
    );

    // Construct a queue.
    let mut queue = context
        .queue_observer()
        .initializing(Uuid::now_v7(), "my_queue", worker);
    // ... queue getting spawned goes here
    queue.operating(None);

    // Construct a memory pool.
    let mut mem_pool =
        context
            .memory_pool_observer()
            .initializing(Uuid::now_v7(), "my_memory_pool", worker);
    // ... pool doing pool things goes here
    mem_pool.operating(1337);

    // Spawn a thread.
    let mut thread = context
        .thread_observer()
        .initializing(Uuid::now_v7(), "my_thread", worker);
    // ... thread getting spawned and handle moved into it goes here
    thread.operating();

    // Single event entity
    context.info_observer().info(
        Uuid::now_v7(),
        "ready to operate".to_string(),
        Some(std::file!().to_string()),
    );

    // Multi-event entities can emit in any order from an entity handle.
    let file_stats_handle = context.file_stats_observer().create(Uuid::now_v7());
    // ... checksum and decompress goes here, can be async as the order of
    // events doesn't matter here
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
    let mut task = context.task_observer().queued(
        Uuid::now_v7(),
        "my_task_31415",
        Ref::new(worker),
        Some(usage((&queue, 1))),
    );

    // ... task sitting in the queue goes here
    task.computing(
        /* thread unit resource usage: */ Some(usage(&thread)),
        /* no memory pool usage: */ None,
    );
    task.computing(Some(usage(&thread)), Some(usage((&mem_pool, 1024))));
    // ... computing goes here
    task.exit();

    Ok(())
}
