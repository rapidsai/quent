// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#include "measure.hpp"

namespace quent_bench {
Measurement measure_empty(const Workload &workload) {
  struct Empty {};
  return measure_threads(
      workload, [] { return Empty{}; }, [](std::uint64_t) { return Empty{}; },
      [](const Empty &, Empty) {
        asm volatile("" ::: "memory");
      });
}
} // namespace quent_bench
