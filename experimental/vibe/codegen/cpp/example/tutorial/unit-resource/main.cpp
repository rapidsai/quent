/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-tutorial-unit-resource-cpp-bridge/gen/quent.hpp"

int main() {
  auto context = quent::Context::none();
  auto pool = context.thread_pool_observer()->handle();
  pool.created();

  auto thread = context.thread_observer()->handle();
  thread.registered(quent::thread::Registered{.pool = pool.id()});

  auto running = context.task_observer()->handle().running(quent::task::Running{
      .thread =
          quent::refs::ThreadUsageRef{
              .target = thread.id(),
              .data = quent::records::ThreadUsage{},
          },
  });
  auto completed = std::move(running).completed();
  return 0;
}
