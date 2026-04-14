// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_attributes::CustomAttributes;
use quent_model::{Attributes, Ref, entity, fsm, instrumentation, model, resource, state};
use serde::{Deserialize, Serialize};

// A "unit" resource.
//
// Only one entity may use this at a time.
//
// Inline doc strings are kept.
resource! {
    /// A thread running tasks.
    Thread
}

// A resource with a capacity.
//
// Multiple entities may use this at a time.
//
// A Resource has a pre-defined FSM:
//
// initializing -> operating -> finalizing -> exit
//
// The maximum capacity is set in the transition to operating.
resource! {
    /// A cache holding on to recent things.
    Cache {
        capacity: { slots: u64 },
    }
}

// A resource with capacities that are resizable at run-time.
//
// To this end, it will have an additional state compared to fixed-capacity
// resources, for when it's resizing:
//
// initializing -> operating <-> resizing -> finalizing -> exit
resource! {
    /// A memory pool providing space to do things.
    MemoryPool {
        resizable: true,
        capacity: { bytes: u64 },
    }
}

// A resource with a potentially unbounded capacity.
//
// In the instrumentation API, in the operating state, these fields
// can be set to None at run-time.
resource! {
    /// A queue to enqueue stuff.
    Queue {
        capacity: { entries: Option<u64> },
    }
}

// A trivial single-event entity.
//
// Note that this is a demonstration of how Quent can even be used to sink
// structured logs.
entity! {
    /// An info message.
    Info {
        attributes: {
            message: String,
            source: Option<String>,
        },
    }
}

// Attributes for a multi-event entity.
/// Details of the applied checksum.
#[derive(Attributes, Serialize, Deserialize)]
pub struct Checksum {
    pub algorithm: String,
    pub value: String,
}

/// Details of the decompression stage.
#[derive(Attributes, Serialize, Deserialize)]
pub struct Decompressed {
    pub algorithm: String,
    pub ratio: f64,
}

// A multi-event entity.
//
// Events are considered unordered. This is useful for grouping events where
// their timestamps don't have a clear relation (like in FSM state transitions).
// For example, when recording the outcome of two pieces of asynchronous work
// without having to necessarily synchronize within the application (as far as
// emitting these events is concerned).
entity! {
    FileStats {
        events: {
            checksum: Checksum,
            decompressed: Decompressed,
        },
    }
}

// entity! only accepts either events for multi-event entities, or attributes
// for a single-event entity. Uncommenting the below will result in an error:

// entity! {
//     BrokenFileStats {
//         attributes: {
//             nope: u64,
//         },
//         events: {
//             checksum: Checksum,
//             decompressed: Decompressed,
//         },
//     }
// }

// The above will cause the compiler complain:
//
// cannot combine `attributes` and `events` on a non-resource-group entity; use
// either `attributes: { ... }` (self-event) or `events: { ... }` (multi-event)

// Structs with key-value attributes
#[derive(Attributes, Serialize, Deserialize)]
pub struct Details {
    pub version: String,          // key known at compile-time
    pub custom: CustomAttributes, // for keys known at run-time only
}

// An entity can be marked as a Resource Group.
//
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

// A multi-event entity that is also a resource group
// must carry the required resource group attributes
// in one of the events marked with `declaration`:
#[derive(Attributes, Serialize, Deserialize)]
pub struct MyEvent {}
entity! {
    Example: ResourceGroup {
        events: {
            a: MyEvent,
            b: MyEvent,
        },
        declaration: a,
    }
}

// An FSM state.
//
// Can have attributes and resource usages.
state! {
    Queued {
        attributes: {
            index: u64,
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

// An FSM.
//
// Must declare its states, its entry state, and the states from which it can
// exit, and its possible transitions.
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
//
// There must always be exactly one root resource group. This requirement exists
// in order to provide an entry-point for a top-down analysis flow, which starts
// at the UUID of this root resource group.
//
// If we do not supply a root, we would get the following error:
//
// ```
// model! requires at least a root resource group
// ```
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
    // ... pool being used goes here
    mem_pool.resizing();
    // ... whoops it's not large enough, we have to resize (usages are allowed
    // to continue during resize)
    mem_pool.operating(2048);
    // ... operating again at a larger capacity.

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
    // ... checksum and decompress goes here, can be emitted async as the order of
    // events doesn't matter here
    // TODO(johanpel): address the issue of sharing the handle across thread boundaries
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
        1,
        Ref::new(worker),
        Some(usage((&queue, 1))),
    );

    // ... task sitting in the queue goes here
    task.computing(
        /* thread usage: */ Some(usage(&thread)),
        /* no memory pool usage: */ None,
    );
    task.computing(
        /* thread usage: */ Some(usage(&thread)),
        /* memory pool usage: */ Some(usage((&mem_pool, 1024))),
    );
    // ... computing goes here
    task.exit();

    Ok(())
}
