// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_model::{
    Attributes, Ref, attributes::CustomAttributes, entity, fsm, instrumentation, model, resource,
    state,
};
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
