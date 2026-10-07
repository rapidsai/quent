// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#include "measure.hpp"
#include "quent-bench-cpp-quent/gen/all/quent.hpp"
#include "quent-bench-cpp-quent/gen/empty/quent.hpp"
#include "quent-bench-cpp-quent/gen/long_string/quent.hpp"
#include "quent-bench-cpp-quent/gen/short_string/quent.hpp"
#include "quent-bench-cpp-quent/gen/u64/quent.hpp"
#include "quent-bench-cpp-quent/gen/u8/quent.hpp"
#include "quent.hpp"

#include <stdexcept>
#include <string>
#include <string_view>

namespace quent_bench {
namespace {

std::string short_value(std::uint64_t index) {
  return std::string(8 + index % 9, 's');
}

std::string long_value(std::uint64_t index) {
  return std::string(128 + index % 129, 'l');
}

template <typename Context>
Context make_context(std::string_view exporter, const std::string &output) {
  if (exporter == "noop")
    return Context::none();
  if (exporter == "ndjson")
    return Context::ndjson(output);
  if (exporter == "msgpack")
    return Context::msgpack(output);
  if (exporter == "postcard")
    return Context::postcard(output);
  throw std::invalid_argument("unknown exporter");
}

template <typename Context, typename Prepare, typename Emit>
Measurement run(const Workload &workload, std::string_view exporter,
                const std::string &output, Prepare prepare, Emit emit) {
  auto context = make_context<Context>(exporter, output);
  auto observer = context.entity_observer();
  auto result = measure_threads(
      workload, [&] { return observer->handle(); }, prepare, emit);
  result.context_id = rust::String(to_string(context.id()));
  return result;
}

} // namespace

Measurement measure_quent(const Workload &workload, rust::Str shape_arg,
                          rust::Str exporter_arg, rust::Str output_arg) {
  const std::string_view shape(shape_arg.data(), shape_arg.size());
  const std::string_view exporter(exporter_arg.data(), exporter_arg.size());
  const std::string output(output_arg.data(), output_arg.size());
  if (shape == "empty") {
    struct Empty {};
    return run<empty::Context>(
        workload, exporter, output, [](std::uint64_t) { return Empty{}; },
        [](const auto &handle, Empty) { handle.instr_call(); });
  }
  const auto emit = [](const auto &handle, auto payload) {
    handle.instr_call(std::move(payload));
  };
  if (shape == "u8") {
    return run<u8::Context>(
        workload, exporter, output,
        [](std::uint64_t i) {
          return u8::entity::InstrCall{static_cast<std::uint8_t>(i)};
        },
        emit);
  }
  if (shape == "u64") {
    return run<u64::Context>(
        workload, exporter, output,
        [](std::uint64_t i) { return u64::entity::InstrCall{i}; }, emit);
  }
  if (shape == "short-string") {
    return run<short_string::Context>(
        workload, exporter, output,
        [](std::uint64_t i) {
          return short_string::entity::InstrCall{short_value(i)};
        },
        emit);
  }
  if (shape == "long-string") {
    return run<long_string::Context>(
        workload, exporter, output,
        [](std::uint64_t i) {
          return long_string::entity::InstrCall{long_value(i)};
        },
        emit);
  }
  if (shape == "all") {
    return run<all::Context>(
        workload, exporter, output,
        [](std::uint64_t i) {
          return all::entity::InstrCall{static_cast<std::uint8_t>(i), i,
                                        short_value(i), long_value(i)};
        },
        emit);
  }
  throw std::invalid_argument("unknown event shape");
}
} // namespace quent_bench
