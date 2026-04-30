// Example: C++ application using quent model-generated instrumentation API.
//
// This exercises the same model as the Rust README example:
// Cluster (root) -> Worker -> Resources (Thread, Queue, MemoryPool)
// Entities: Info (single-event), FileStats (multi-event)
// FSM: Task (queued -> computing)

#include "quent-bridge/gen/cluster.rs.h"
#include "quent-bridge/gen/context.rs.h"
#include "quent-bridge/gen/custom_attributes.rs.h"
#include "quent-bridge/gen/file_stats.rs.h"
#include "quent-bridge/gen/info.rs.h"
#include "quent-bridge/gen/memory_pool.rs.h"
#include "quent-bridge/gen/queue.rs.h"
#include "quent-bridge/gen/task.rs.h"
#include "quent-bridge/gen/thread.rs.h"
#include "quent-bridge/gen/uuid.rs.h"
#include "quent-bridge/gen/worker.rs.h"

#include <filesystem>
#include <iostream>
#include <string>

int main() {
  // The root resource group uses the same ID as the context.
  auto cluster_id = uuid::now_v7();
  {
    // Create instrumentation context — events exported to ndjson.
    auto ctx = quent::create_context(cluster_id, "ndjson", "./events");

    auto cluster_obs = quent::cluster::create_observer();
    cluster_obs->cluster_declaration(cluster_id,
                                     quent::cluster::ClusterDeclaration{
                                         .instance_name = "example_cluster",
                                     });

    // Spawn a worker.
    auto worker_obs = quent::worker::create_observer();
    auto worker_id = uuid::now_v7();
    quent::CustomAttributes custom;
    custom.string_attrs.push_back({"version", "42.1.2"});
    custom.i64_attrs.push_back({"threads", 256});
    worker_obs->worker_declaration(worker_id,
                                   quent::worker::WorkerDeclaration{
                                       .instance_name = "worker_0",
                                       .cluster = cluster_id,
                                       .details =
                                           quent::worker::Details{
                                               .version = "42.1.2",
                                               .custom = std::move(custom),
                                           },
                                   });

    // Construct a queue.
    auto queue = quent::queue::create(quent::queue::Initializing{
        .instance_name = "my_queue",
        .parent_group_id = worker_id,
    });
    // Operating with unbounded capacity (Option<u64> = None in Rust).
    // In C++, the bridge wraps the value in Some(), so 0 here means Some(0).
    // True unbounded (None) is not representable in CXX without sentinel
    // support.
    queue->operating(quent::queue::Operating{.capacity_entries = 0});

    // Construct a memory pool (resizable).
    auto mem_pool = quent::memory_pool::create(quent::memory_pool::Initializing{
        .instance_name = "my_memory_pool",
        .parent_group_id = worker_id,
    });
    mem_pool->operating(quent::memory_pool::Operating{.capacity_bytes = 1337});
    mem_pool->resizing();
    mem_pool->operating(quent::memory_pool::Operating{.capacity_bytes = 2048});

    // Spawn a thread.
    auto thread = quent::thread::create(quent::thread::Initializing{
        .instance_name = "my_thread",
        .parent_group_id = worker_id,
    });
    thread->operating();

    // Single event entity.
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

    // Queue a task. The entry transition returns an FSM handle.
    auto task = quent::task::create(quent::task::Queued{
        .instance_name = "my_task_31415",
        .index = 1,
        .worker = worker_id,
        .queue_resource_id = queue->uuid(),
    });

    // First computing transition — thread usage only, no memory pool.
    task->computing(quent::task::Computing{
        .thread_resource_id = thread->uuid(),
        .memory_resource_id = uuid::new_nil(),
    });

    // Second computing transition — both thread and memory pool.
    task->computing(quent::task::Computing{
        .thread_resource_id = thread->uuid(),
        .memory_resource_id = mem_pool->uuid(),
    });

    // Task exits.
    task->exit();

  } // Drop context to flush all pending events.

  auto output_path = std::filesystem::canonical("./events").string() + "/" +
                     std::string(uuid::to_string(cluster_id)) + ".ndjson";
  std::cout << "Events written to: " << output_path << std::endl;

  return 0;
}
