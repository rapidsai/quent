/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-nvtx-cpp-test/gen/quent.hpp"

#include <fstream>
#include <optional>
#include <type_traits>

extern "C" void nvtxMarkA(const char*);

static_assert(std::is_move_constructible_v<quent::NvtxCapture>);
static_assert(!std::is_copy_constructible_v<quent::NvtxCapture>);

extern "C" int quent_nvtx_cpp_run(const char* output) {
  try {
    std::optional<quent::NvtxCapture> capture;
    {
      auto context = quent::Context::ndjson(output);
      auto process = context.process_observer()->handle();
      process.started({{42}});
      std::ofstream(std::string(output) + "/context-id") << quent::to_string(context.id());
      std::ofstream(std::string(output) + "/process-id") << quent::to_string(process.id().raw());
      auto observer = context.quent_nvtx_sink_observer();
      capture.emplace(quent::NvtxCapture::install(*observer, process.id()));
      nvtxMarkA("before-context-drop");
    }
    nvtxMarkA("after-context-drop");
    capture.reset();
    nvtxMarkA("after-capture-drop");
    return 0;
  } catch (const std::exception& error) {
    std::ofstream(std::string(output) + "/error") << error.what();
    return 1;
  }
}
