/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "quent-nvtx-cpp-test/gen/quent.hpp"

#include <cstdint>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <type_traits>

// Use the NVTX C entry points linked by nvtx-sys.
extern "C" {
void nvtxMarkA(const char *);
int nvtxRangePushA(const char *);
int nvtxRangePop();
std::uint64_t nvtxRangeStartA(const char *);
void nvtxRangeEnd(std::uint64_t);
}

static_assert(std::is_same_v<quent::NvtxCapture, quent::detail::NvtxCapture>);

static void remember_id(const std::string &root, const quent::Uuid &id) {
  std::ofstream(root + "/context-id") << quent::to_string(id);
}

static void annotate() {
  nvtxMarkA("captured-mark");
  nvtxRangePushA("captured-push");
  nvtxRangePop();
  const auto range = nvtxRangeStartA("captured-start");
  nvtxRangeEnd(range);
}

static quent::Context filesystem(const std::string &format,
                                 const std::string &root) {
  if (format == "ndjson")
    return quent::Context::ndjson(root, quent::NvtxCapture::Enabled);
  if (format == "msgpack")
    return quent::Context::msgpack(root, quent::NvtxCapture::Enabled);
  if (format == "postcard")
    return quent::Context::postcard(root, quent::NvtxCapture::Enabled);
  throw std::runtime_error("unknown filesystem format");
}

// Compile both overload forms of every exporter, including collector. Runtime
// collector capture needs a receiver with an NvtxEvent route and is not tested.
[[maybe_unused]] static void collector_contract() {
  auto default_capture = quent::Context::collector("http://127.0.0.1:1");
  auto explicit_capture = quent::Context::collector(
      "http://127.0.0.1:1", quent::NvtxCapture::Enabled);
}
[[maybe_unused]] static void filesystem_defaults(const std::string &root) {
  auto msgpack = quent::Context::msgpack(root);
  auto postcard = quent::Context::postcard(root);
}

extern "C" int quent_nvtx_cpp_run(const char *scenario_c, const char *root_c) {
  try {
    const std::string scenario(scenario_c);
    const std::string root(root_c);
    if (scenario == "disabled") {
      {
        auto default_capture = quent::Context::ndjson(root + "/default");
        nvtxMarkA("disabled-default");
        auto explicit_capture = quent::Context::ndjson(
            root + "/explicit", quent::NvtxCapture::Disabled);
        nvtxMarkA("disabled-explicit");
        auto raw = quent::detail::create_context(
            quent::detail::ExporterOptions::ndjson(root + "/raw"),
            quent::detail::NvtxCapture::Disabled);
        nvtxMarkA("disabled-raw");
        auto noop = quent::Context::none();
        auto raw_noop = quent::detail::create_context(
            quent::detail::ExporterOptions::none(),
            quent::detail::NvtxCapture::Enabled);
        nvtxMarkA("disabled-noop");
      }
      // Disabled/no-op creation must not consume the one-shot installation.
      auto enabled = filesystem("ndjson", root + "/enabled");
      remember_id(root + "/enabled", enabled.id());
      annotate();
      return 0;
    }
    if (scenario == "raw") {
      {
        auto context = quent::detail::create_context(
            quent::detail::ExporterOptions::ndjson(root),
            quent::detail::NvtxCapture::Enabled);
        remember_id(root, context->id());
        annotate();
      }
      nvtxMarkA("after-context-drop");
      return 0;
    }
    std::shared_ptr<quent::cluster::ClusterObserver> surviving_observer;
    {
      auto context = filesystem(
          scenario == "duplicate" || scenario == "retained" ? "ndjson"
                                                            : scenario,
          root);
      remember_id(root, context.id());
      auto observer = context.cluster_observer();
      if (scenario == "retained")
        surviving_observer = observer;
      auto cluster = observer->handle(quent::cluster::ClusterId(context.id()));
      cluster.declaration(
          quent::cluster::Declaration{.instance_name = "nvtx-fixture"});
      annotate();
      if (scenario == "duplicate") {
        bool rejected = false;
        try {
          auto second = quent::Context::ndjson(root + "/second",
                                               quent::NvtxCapture::Enabled);
        } catch (const rust::Error &) {
          rejected = true;
        }
        if (!rejected)
          return 2;
        nvtxMarkA("first-still-active");
      }
    }
    // Context teardown must stop capture and synchronously flush the exporter.
    nvtxMarkA("after-context-drop");
    surviving_observer.reset();
    return 0;
  } catch (const std::exception &error) {
    std::cerr << "NVTX fixture failed: " << error.what() << '\n';
    return 1;
  }
}
