/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-demo-cpp-bridge/gen/quent.hpp"

#include <type_traits>

static_assert(
    !std::is_same_v<quent::cluster::ClusterId, quent::worker::WorkerId>);
static_assert(
    !std::is_convertible_v<quent::cluster::ClusterId,
                           quent::worker::WorkerId>);

int run_example() {
#ifdef QUENT_DEMO_LIBRARY
  auto context = quent::Context::none();
#else
  auto context = quent::Context::ndjson("./events");
#endif

  auto cluster_observer = context.cluster_observer();
  auto scoped_cluster_telemetry = cluster_observer;
  auto cluster = scoped_cluster_telemetry->create(
      quent::cluster::ClusterId(context.id()));
  cluster.declaration(
      quent::cluster::Declaration{.instance_name = "example_cluster"});
  if (!cluster.declaration_emitted()) return 1;
  try {
    cluster.declaration(
        quent::cluster::Declaration{.instance_name = "duplicate_cluster"});
    return 2;
  } catch (const rust::Error &) {
  }

  quent::DynamicAttributes custom;
  custom.add_u64("threads", 256);
  auto worker = context.worker_observer()->create();
  worker.declaration(quent::worker::Declaration{
      .instance_name = "worker_0",
      .cluster = cluster.id(),
      .details = quent::records::Details{
          .version = "42.1.2",
          .custom = std::move(custom),
      },
  });

  auto queue = context.queue_observer()->create();
  queue.declaration(quent::queue::Declaration{
      .instance_name = "my_queue",
      .worker = worker.id(),
  });

  auto memory = context.memory_pool_observer()->create();
  memory.declaration(quent::memory_pool::Declaration{
      .instance_name = "my_memory_pool",
      .worker = worker.id(),
      .limits = quent::records::MemoryPoolBounds{.bytes = 1337},
  });
  memory.resized(quent::memory_pool::Resized{
      .limits = quent::records::MemoryPoolBounds{.bytes = 2048},
  });

  auto thread = context.thread_observer()->create();
  thread.idle(quent::thread::Idle{.worker = worker.id()});
  thread.active();

  auto info = context.info_observer()->create();
  info.recorded(quent::info::Recorded{
      .message = "ready to operate",
      .source = std::string(__FILE__),
      .worker = worker.id(),
  });

  auto file_stats = context.file_stats_observer()->create();
  file_stats.scheduled();
  file_stats.checksum(quent::file_stats::Checksum{
      .details = quent::records::Checksum{
          .algorithm = "sha256",
          .value = "abc123def456",
      },
      .worker = worker.id(),
  });
  file_stats.decompressed(quent::file_stats::Decompressed{
      .details = quent::records::Decompressed{
          .algorithm = "snappy",
          .ratio = 0.4,
      },
  });

  auto task = context.task_observer()->create();
  task.queued(quent::task::Queued{
      .instance_name = "my_task_31415",
      .index = 1,
      .worker = worker.id(),
      .use_queue = quent::refs::QueueUsageRef{
          .target = queue.id(),
          .data = quent::records::QueueUsage{.entries = 1},
      },
  });
  task.computing(quent::task::Computing{
      .use_thread = quent::refs::ThreadUsageRef{
          .target = thread.id(),
          .data = quent::records::ThreadUsage{},
      },
      .use_memory = std::nullopt,
  });
  task.computing(quent::task::Computing{
      .use_thread = quent::refs::ThreadUsageRef{
          .target = thread.id(),
          .data = quent::records::ThreadUsage{},
      },
      .use_memory = quent::refs::MemoryPoolUsageRef{
          .target = memory.id(),
          .data = quent::records::MemoryPoolUsage{.bytes = 1024},
      },
  });
  task.exit();
  thread.idle(quent::thread::Idle{.worker = worker.id()});
  thread.exit();

  auto detached_observer = [] {
    auto detached_context = quent::Context::none();
    return detached_context.cluster_observer();
  }();
  auto detached_cluster = detached_observer->create();
  detached_cluster.declaration(
      quent::cluster::Declaration{.instance_name = "detached_cluster"});
  return 0;
}

#ifdef QUENT_DEMO_LIBRARY
extern "C" int quent_demo_cpp_smoke() { return run_example(); }
#else
int main() { return run_example(); }
#endif
