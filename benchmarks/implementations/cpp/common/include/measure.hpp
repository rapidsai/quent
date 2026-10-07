// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#pragma once

#include "quent-bench-cpp-common/src/lib.rs.h"

#include <barrier>
#include <chrono>
#include <exception>
#include <thread>
#include <utility>
#include <vector>

namespace quent_bench {

using Clock = std::chrono::steady_clock;

inline void busy_wait(std::uint64_t interval_us) {
  const auto start = Clock::now();
  while (static_cast<std::uint64_t>(
             std::chrono::duration_cast<std::chrono::microseconds>(
                 Clock::now() - start)
                 .count()) < interval_us) {
  }
}

template <typename MakeHandle, typename Prepare, typename Emit>
Measurement measure_threads(const Workload &workload, MakeHandle make_handle,
                            Prepare prepare, Emit emit) {
  using Handle = decltype(make_handle());
  using Payload = decltype(prepare(std::uint64_t{}));
  const auto total_batches = workload.num_warmup_batches + workload.num_batches;
  std::vector<Handle> handles;
  handles.reserve(workload.threads);
  for (std::size_t thread = 0; thread < workload.threads; ++thread) {
    handles.push_back(make_handle());
  }
  std::barrier barrier(static_cast<std::ptrdiff_t>(workload.threads));
  std::vector<std::exception_ptr> errors(workload.threads);
  std::vector<std::vector<std::uint64_t>> durations(workload.threads);
  std::vector<std::jthread> workers;
  workers.reserve(workload.threads);
  try {
    for (std::size_t thread = 0; thread < workload.threads; ++thread) {
      workers.emplace_back([&, thread, handle = std::move(handles[thread])] {
        try {
          if (workload.preflight_call) {
            emit(handle, prepare(0));
          }
          std::vector<std::vector<Payload>> batches;
          batches.reserve(total_batches);
          for (std::size_t batch = 0; batch < total_batches; ++batch) {
            auto &payloads = batches.emplace_back();
            payloads.reserve(workload.batch_size);
            for (std::size_t index = 0; index < workload.batch_size; ++index) {
              payloads.push_back(prepare(batch * workload.batch_size + index));
            }
          }
          auto &samples = durations[thread];
          samples.reserve(workload.num_batches);
          barrier.arrive_and_wait();
          for (std::size_t batch = 0; batch < total_batches; ++batch) {
            auto payloads = std::move(batches[batch]);
            barrier.arrive_and_wait();
            if (batch < workload.num_warmup_batches) {
              for (auto &payload : payloads) {
                emit(handle, std::move(payload));
              }
            } else {
              const auto start = Clock::now();
              for (auto &payload : payloads) {
                emit(handle, std::move(payload));
              }
              const auto elapsed = Clock::now() - start;
              samples.push_back(
                  std::chrono::duration_cast<std::chrono::nanoseconds>(elapsed)
                      .count());
            }
            // Release batch storage outside timing, before the completion
            // barrier.
            std::vector<Payload>().swap(payloads);
            barrier.arrive_and_wait();
            if (batch + 1 < total_batches && workload.batch_pause_us > 0) {
              busy_wait(workload.batch_pause_us);
            }
          }
        } catch (...) {
          errors[thread] = std::current_exception();
          // Other workers must be able to finish if one call or allocation
          // fails.
          barrier.arrive_and_drop();
        }
      });
    }
  } catch (...) {
    for (auto missing = workers.size(); missing < workload.threads; ++missing) {
      barrier.arrive_and_drop();
    }
    throw;
  }
  for (auto &worker : workers) {
    worker.join();
  }
  for (const auto &error : errors) {
    if (error) {
      std::rethrow_exception(error);
    }
  }
  Measurement result;
  result.elapsed_ns.reserve(workload.threads * workload.num_batches);
  for (const auto &samples : durations) {
    for (const auto elapsed : samples) {
      result.elapsed_ns.push_back(elapsed);
    }
  }
  return result;
}
} // namespace quent_bench
