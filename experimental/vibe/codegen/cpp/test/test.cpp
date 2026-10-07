/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include QUENT_CPP_BRIDGE_HEADER

#include <type_traits>
#include <utility>

using JobQueued = quent::FsmHandle<quent::Job, quent::job_state::Queued>;
using JobRunning = quent::FsmHandle<quent::Job, quent::job_state::Running>;
using JobDynamic = quent::DynamicFsmHandle<quent::Job>;

template <typename Handle>
concept CanFinish = requires(Handle handle) { std::move(handle).done(); };

template <typename Handle>
concept CanDeclareWorker =
    requires(Handle &handle, quent::worker::Declaration event) {
      handle.declaration(std::move(event));
    };

static_assert(std::is_same_v<
              decltype(std::declval<const quent::worker::WorkerObserver &>()
                           .handle()),
              quent::Handle<quent::Worker>>);
static_assert(CanDeclareWorker<quent::Handle<quent::Worker>>);
static_assert(!CanDeclareWorker<quent::Handle<quent::Cluster>>);
static_assert(!std::is_copy_constructible_v<quent::Handle<quent::Worker>>);
static_assert(std::is_move_constructible_v<quent::Handle<quent::Worker>>);
static_assert(
    std::is_same_v<decltype(std::declval<quent::FsmHandle<quent::Job>>().queued(
                       std::declval<quent::job::Queued>())),
                   JobQueued>);
static_assert(!CanFinish<JobQueued>);
static_assert(CanFinish<JobRunning>);
static_assert(CanFinish<JobDynamic>);
static_assert(std::is_same_v<
              decltype(std::declval<JobRunning>().into_dynamic()), JobDynamic>);
static_assert(
    std::is_same_v<decltype(std::declval<JobDynamic>()
                                .try_into<quent::job_state::Running>()),
                   std::optional<JobRunning>>);

std::optional<JobRunning> recover_running(JobDynamic job) {
  return std::move(job).try_into<quent::job_state::Running>();
}

quent::DynamicAttributes make_dynamic_attributes();
int run_example();

extern "C" int quent_demo_cpp_smoke() { return run_example(); }

extern "C" int quent_demo_cpp_dynamic_try_into() {
  auto context = quent::Context::none();
  auto dynamic = context.job_observer()->handle().into_dynamic();
  auto running = std::move(dynamic).try_into<quent::job_state::Running>();
  if (running) {
    return 1;
  }
  auto initial =
      std::move(dynamic).try_into<quent::facade_detail::InitialFsmState>();
  return initial ? 0 : 2;
}

extern "C" int quent_demo_cpp_dynamic_values(const char *output_dir) {
  auto context = quent::Context::ndjson(output_dir);
  auto worker = context.worker_observer()->handle();
  worker.declaration(quent::worker::Declaration{
      .instance_name = "dynamic_values",
      .cluster = quent::cluster::ClusterId(context.id()),
      .details =
          quent::records::Details{
              .version = "1.0",
              .custom = make_dynamic_attributes(),
          },
  });
  return 0;
}
