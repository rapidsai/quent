/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-codegen-cpp-nvtx-test/gen/quent.hpp"

#include <cstdint>
#include <limits>

#if defined(QUENT_NVTX_LIVE_CAPTURE)
extern "C" void nvtxMarkA(const char *message);
extern "C" int nvtxRangePushA(const char *message);
extern "C" int nvtxRangePop();

extern "C" int quent_codegen_cpp_nvtx_capture(const char *output_dir,
                                               std::uint32_t process_id) {
  auto context = quent::Context::ndjson(output_dir);
  auto observer = context.process_observer();
  auto process = observer->handle();

  const auto foreign_process_id =
      process_id == std::numeric_limits<std::uint32_t>::max()
          ? process_id - 1
          : process_id + 1;
  try {
    process.started(
        quent::process::Started{.process = {.native_id = foreign_process_id}});
    return 1;
  } catch (const rust::Error &) {
  }
  if (process.started_emitted()) {
    return 2;
  }

  try {
    process.started(
        quent::process::Started{.process = {.native_id = process_id}});
  } catch (const rust::Error &) {
    return 3;
  }

  nvtxMarkA("cpp-generated-mark");
  nvtxRangePushA("cpp-generated-range");
  nvtxRangePop();
  return 0;
}
#endif
