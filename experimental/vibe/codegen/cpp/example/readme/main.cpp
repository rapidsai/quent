/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#ifndef QUENT_CPP_BRIDGE_HEADER
#define QUENT_CPP_BRIDGE_HEADER "quent-demo-cpp-bridge/gen/quent.hpp"
#endif
#include QUENT_CPP_BRIDGE_HEADER

#include <type_traits>

static_assert(
    !std::is_same_v<quent::cluster::ClusterId, quent::worker::WorkerId>);
static_assert(
    !std::is_convertible_v<quent::cluster::ClusterId, quent::worker::WorkerId>);

quent::DynamicAttributes make_dynamic_attributes() {
  quent::DynamicAttributes custom;
  custom.add("null", nullptr);
  custom.add("bool", true);
  custom.add("u8", std::uint8_t{8});
  custom.add("u16", std::uint16_t{16});
  custom.add("u32", std::uint32_t{32});
  custom.add("u64", std::uint64_t{64});
  custom.add("i8", std::int8_t{-8});
  custom.add("i16", std::int16_t{-16});
  custom.add("i32", std::int32_t{-32});
  custom.add("i64", std::int64_t{-64});
  custom.add("f32", 32.5F);
  custom.add("f64", 64.5);
  custom.add("string", "value");

  quent::DynamicAttributes structure;
  structure.add("first", "alpha");
  structure.add("second", std::uint32_t{2});
  custom.add("structure", std::move(structure));

  custom.add("u8_list", std::vector<std::uint8_t>{1, 2});
  custom.add("u16_list", std::vector<std::uint16_t>{1, 2});
  custom.add("u32_list", std::vector<std::uint32_t>{1, 2});
  custom.add("u64_list", std::vector<std::uint64_t>{1, 2});
  custom.add("i8_list", std::vector<std::int8_t>{-1, 2});
  custom.add("i16_list", std::vector<std::int16_t>{-1, 2});
  custom.add("i32_list", std::vector<std::int32_t>{-1, 2});
  custom.add("i64_list", std::vector<std::int64_t>{-1, 2});
  custom.add("f32_list", std::vector<float>{1.5F, 2.5F});
  custom.add("f64_list", std::vector<double>{1.5, 2.5});
  custom.add("string_list", std::vector<std::string>{"first", "second"});

  std::vector<quent::DynamicAttributes> structures;
  quent::DynamicAttributes first_structure;
  first_structure.add("name", "first");
  structures.push_back(std::move(first_structure));
  quent::DynamicAttributes second_structure;
  second_structure.add("name", "second");
  structures.push_back(std::move(second_structure));
  custom.add("struct_list", std::move(structures));

  std::vector<quent::DynamicList> inner_lists;
  inner_lists.push_back(quent::DynamicList::string({"nested"}));
  std::vector<quent::DynamicList> nested_lists;
  nested_lists.push_back(quent::DynamicList::u8({1, 2}));
  nested_lists.push_back(quent::DynamicList::list(std::move(inner_lists)));
  custom.add("nested_list", quent::DynamicList::list(std::move(nested_lists)));

  quent::DynamicAttributes moved_from;
  moved_from.add("before_move", "retained");
  auto moved_to = std::move(moved_from);
  moved_from.add("after_move", "valid");
  custom.add("moved_to", std::move(moved_to));
  custom.add("moved_from", std::move(moved_from));
  return custom;
}

int run_example() {
#ifdef QUENT_DEMO_LIBRARY
  auto context = quent::Context::none();
#else
  auto context = quent::Context::ndjson("./events");
#endif

  auto cluster_observer = context.cluster_observer();
  auto scoped_cluster_telemetry = cluster_observer;
  auto cluster =
      scoped_cluster_telemetry->handle(quent::cluster::ClusterId(context.id()));
  cluster.declaration(
      quent::cluster::Declaration{.instance_name = "example_cluster"});
  if (!cluster.declaration_emitted())
    return 1;
  try {
    cluster.declaration(
        quent::cluster::Declaration{.instance_name = "duplicate_cluster"});
    return 2;
  } catch (const rust::Error &) {
  }

  auto custom = make_dynamic_attributes();
  auto worker = context.worker_observer()->handle();
  worker.declaration(quent::worker::Declaration{
      .instance_name = "worker_0",
      .cluster = cluster.id(),
      .details =
          quent::records::Details{
              .version = "42.1.2",
              .custom = std::move(custom),
          },
  });

  auto queue = context.queue_observer()->handle();
  queue.declaration(quent::queue::Declaration{
      .instance_name = "my_queue",
      .worker = worker.id(),
  });

  auto memory = context.memory_pool_observer()->handle();
  memory.declaration(quent::memory_pool::Declaration{
      .instance_name = "my_memory_pool",
      .worker = worker.id(),
      .limits = quent::records::MemoryPoolBounds{.bytes = 1337},
  });
  memory.resized(quent::memory_pool::Resized{
      .limits = quent::records::MemoryPoolBounds{.bytes = 2048},
  });

  auto thread = context.thread_observer()
                    ->handle()
                    .idle(quent::thread::Idle{.worker = worker.id()})
                    .active();

  auto info = context.info_observer()->handle();
  info.recorded(quent::info::Recorded{
      .message = "ready to operate",
      .source = std::string(__FILE__),
      .worker = worker.id(),
  });

  auto file_stats = context.file_stats_observer()->handle();
  file_stats.scheduled();
  file_stats.checksum(quent::file_stats::Checksum{
      .details =
          quent::records::Checksum{
              .algorithm = "sha256",
              .value = "abc123def456",
          },
      .worker = worker.id(),
  });
  file_stats.decompressed(quent::file_stats::Decompressed{
      .details =
          quent::records::Decompressed{
              .algorithm = "snappy",
              .ratio = 0.4,
          },
  });

  context.task_observer()
      ->handle()
      .queued(quent::task::Queued{
          .instance_name = "my_task_31415",
          .index = 1,
          .worker = worker.id(),
          .use_queue =
              quent::refs::QueueUsageRef{
                  .target = queue.id(),
                  .data = quent::records::QueueUsage{.entries = 1},
              },
      })
      .computing(quent::task::Computing{
          .use_thread =
              quent::refs::ThreadUsageRef{
                  .target = thread.id(),
                  .data = quent::records::ThreadUsage{},
              },
          .use_memory = std::nullopt,
      })
      .computing(quent::task::Computing{
          .use_thread =
              quent::refs::ThreadUsageRef{
                  .target = thread.id(),
                  .data = quent::records::ThreadUsage{},
              },
          .use_memory =
              quent::refs::MemoryPoolUsageRef{
                  .target = memory.id(),
                  .data = quent::records::MemoryPoolUsage{.bytes = 1024},
              },
      })
      .exit();
  auto idle_thread =
      std::move(thread).idle(quent::thread::Idle{.worker = worker.id()});
  std::move(idle_thread).exit();

  auto detached_observer = [] {
    auto detached_context = quent::Context::none();
    return detached_context.cluster_observer();
  }();
  auto detached_cluster = detached_observer->handle();
  detached_cluster.declaration(
      quent::cluster::Declaration{.instance_name = "detached_cluster"});
  return 0;
}

#ifndef QUENT_DEMO_LIBRARY
int main() { return run_example(); }
#endif
