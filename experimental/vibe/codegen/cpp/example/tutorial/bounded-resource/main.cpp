/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-tutorial-bounded-resource-cpp-bridge/gen/quent.hpp"

int main() {
  auto context = quent::Context::none();
  auto memory = context.memory_observer()->handle();
  memory.resized(quent::memory::Resized{
      .limits = quent::records::MemoryBounds{.bytes = 8'000'000'000},
  });

  auto running = context.task_observer()->handle().running(quent::task::Running{
      .memory =
          quent::refs::MemoryUsageRef{
              .target = memory.id(),
              .data =
                  quent::records::MemoryUsage{
                      .bytes = 512'000'000,
                  },
          },
  });
  auto completed = std::move(running).completed();
  return 0;
}
