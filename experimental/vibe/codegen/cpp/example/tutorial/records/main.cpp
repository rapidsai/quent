/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-tutorial-records-cpp-bridge/gen/quent.hpp"

int main() {
  auto context = quent::Context::none();
  auto task = context.task_observer()->handle();
  auto batch = context.batch_observer()->handle();

  task.started();
  task.ended(quent::task::Ended{
      .result =
          quent::records::WorkResult{
              .success = true,
              .items_processed = 128,
          },
  });

  batch.started();
  batch.ended(quent::batch::Ended{
      .result =
          quent::records::WorkResult{
              .success = true,
              .items_processed = 512,
          },
  });
  return 0;
}
