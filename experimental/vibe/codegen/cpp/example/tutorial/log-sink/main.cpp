/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-tutorial-log-sink-cpp-bridge/gen/quent.hpp"

#include <cstdint>
#include <source_location>
#include <string>
#include <string_view>

using AppLog = quent::Handle<quent::AppLog>;

void info(const AppLog &log, std::string_view message, std::string_view target,
          std::source_location source = std::source_location::current()) {
  log.info(quent::app_log::Info{
      .message = std::string{message},
      .target = std::string{target},
      .file = source.file_name(),
      .line = static_cast<std::uint32_t>(source.line()),
      .module = source.function_name(),
      .thread_name = "main",
  });
}

void warning(const AppLog &log, std::string_view message,
             std::string_view target, std::string_view category,
             std::source_location source = std::source_location::current()) {
  log.warning(quent::app_log::Warning{
      .message = std::string{message},
      .target = std::string{target},
      .file = source.file_name(),
      .line = static_cast<std::uint32_t>(source.line()),
      .module = source.function_name(),
      .thread_name = "main",
      .category = std::string{category},
  });
}

int main() {
  auto context = quent::Context::none();
  auto log = context.app_log_observer()->handle();

  // Default arguments based on std::source_location capture each call site.
  info(log, "application started", "startup");
  warning(log, "retrying request", "network", "transient");

  return 0;
}
