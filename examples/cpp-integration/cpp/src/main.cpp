// Example: C++ application using quent model-generated instrumentation API.
//
// This exercises the same model as the Rust README example:
// Cluster (root) -> Worker -> Resources (Thread, Queue, MemoryPool)
// Entities: Info (single-event), FileStats (multi-event)
// FSM: Task (queued -> computing)

#include "quent-bridge/gen/uuid.rs.h"
#include "quent-bridge/gen/custom_attributes.rs.h"
#include "quent-bridge/gen/context.rs.h"
#include "quent-bridge/gen/cluster.rs.h"
#include "quent-bridge/gen/worker.rs.h"
#include "quent-bridge/gen/thread.rs.h"
#include "quent-bridge/gen/queue.rs.h"
#include "quent-bridge/gen/memory_pool.rs.h"
#include "quent-bridge/gen/info.rs.h"
#include "quent-bridge/gen/file_stats.rs.h"
#include "quent-bridge/gen/task.rs.h"

#include <string>

int main() {
    // Create instrumentation context — events exported to ndjson.
    auto ctx = quent::create_context("ndjson", "data");

    // Declare the cluster (root resource group).
    auto cluster_obs = quent::cluster::create_observer();
    auto cluster_id = uuid::now_v7();
    cluster_obs->cluster_declaration(cluster_id, quent::cluster::ClusterDeclaration{
        .instance_name = "example-cluster",
    });

    // Declare a worker (resource group with typed parent).
    auto worker_obs = quent::worker::create_observer();
    auto worker_id = uuid::now_v7();
    quent::CustomAttributes custom;
    custom.string_attrs.push_back({"version", "42.1.2"});
    custom.i64_attrs.push_back({"threads", 256});
    worker_obs->worker_declaration(worker_id, quent::worker::WorkerDeclaration{
        .instance_name = "worker-0",
        .cluster = cluster_id,
        .details = quent::worker::Details{
            .version = "42.1.2",
            .custom = std::move(custom),
        },
    });

    // Create a thread resource (unit — no capacity).
    auto thread = quent::thread::create(quent::thread::Initializing{
        .instance_name = "my-thread",
        .parent_group_id = worker_id,
    });
    thread->operating();

    // Create a queue resource (optional capacity — 0 sentinel = unbounded).
    auto queue = quent::queue::create(quent::queue::Initializing{
        .instance_name = "my-queue",
        .parent_group_id = worker_id,
    });
    queue->operating(quent::queue::Operating{.capacity_entries = 0});

    // Create a memory pool resource (resizable).
    auto mem_pool = quent::memory_pool::create(quent::memory_pool::Initializing{
        .instance_name = "my-pool",
        .parent_group_id = worker_id,
    });
    mem_pool->operating(quent::memory_pool::Operating{.capacity_bytes = 1337});
    mem_pool->resizing();
    mem_pool->operating(quent::memory_pool::Operating{.capacity_bytes = 2048});

    // Single-event entity: structured log.
    auto info_obs = quent::info::create_observer();
    info_obs->info(uuid::now_v7(), quent::info::Info{
        .message = "ready to operate",
        .source = "main.cpp",
    });

    // Multi-event entity.
    auto file_stats_obs = quent::file_stats::create_observer();
    auto fs_id = uuid::now_v7();
    file_stats_obs->checksum(fs_id, quent::file_stats::Checksum{
        .algorithm = "sha256",
        .value = "abc123def456",
    });
    file_stats_obs->decompressed(fs_id, quent::file_stats::Decompressed{
        .algorithm = "snappy",
        .ratio = 0.4,
    });

    // FSM: queue a task.
    // TODO: resource handles should expose their UUID to C++ so task
    // transitions can reference actual resources (nil = no usage).
    auto task = quent::task::create(quent::task::Queued{
        .instance_name = "my-task",
        .index = 1,
        .worker = worker_id,
        .queue_resource_id = uuid::new_nil(),
    });

    // Transition to computing with resource usages.
    task->computing(quent::task::Computing{
        .thread_resource_id = uuid::new_nil(),
        .memory_resource_id = uuid::new_nil(),
    });

    // Task exits.
    task->exit();

    // Context destruction flushes all pending events.
    return 0;
}
