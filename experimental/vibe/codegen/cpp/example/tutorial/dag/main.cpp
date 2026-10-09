/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-tutorial-dag-cpp-bridge/gen/quent.hpp"

int main() {
  auto context = quent::Context::none();

  auto plan = context.plan_observer()->handle();
  plan.created();

  auto source = context.operator_observer()->handle();
  source.declared(quent::operator_::Declared{.plan = plan.id()});

  auto target = context.operator_observer()->handle();
  target.declared(quent::operator_::Declared{.plan = plan.id()});

  auto edge = context.plan_edge_observer()->handle();
  edge.connected(quent::plan_edge::Connected{
      .plan = plan.id(),
      .source = source.id(),
      .target = target.id(),
  });

  return 0;
}
