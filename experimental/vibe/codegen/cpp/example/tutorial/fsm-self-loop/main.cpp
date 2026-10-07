/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-tutorial-fsm-self-loop-cpp-bridge/gen/quent.hpp"

int main() {
  auto context = quent::Context::none();
  auto task = context.task_observer()->handle().running(
      quent::task::Running{.items_processed = 0});
  task = std::move(task).running(quent::task::Running{.items_processed = 64});
  auto paused = std::move(task).paused();
  task =
      std::move(paused).running(quent::task::Running{.items_processed = 128});
  auto completed = std::move(task).completed();
  return 0;
}
