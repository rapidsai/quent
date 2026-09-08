/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

// Query engine C++ integration test.
//
// Calls every generated CXX bridge function to catch regressions.
// The code does not need to make semantic sense; it exercises the full API.

#include "quent-qe-bridge/gen/context.rs.h"
#include "quent-qe-bridge/gen/dynamic_attributes.rs.h"
#include "quent-qe-bridge/gen/engine.rs.h"
#include "quent-qe-bridge/gen/operator.rs.h"
#include "quent-qe-bridge/gen/plan.rs.h"
#include "quent-qe-bridge/gen/port.rs.h"
#include "quent-qe-bridge/gen/query.rs.h"
#include "quent-qe-bridge/gen/query_group.rs.h"
#include "quent-qe-bridge/gen/uuid.rs.h"
#include "quent-qe-bridge/gen/worker.rs.h"

#include <filesystem>
#include <stdexcept>
#include <string>

namespace {

void emit_single_event(rust::Box<quent::ExporterOptions> options) {
  auto ctx = quent::create_context(std::move(options));
  auto query_group = quent::query_group::create_observer(*ctx);
  query_group->declaration(
      uuid::now_v7(),
      quent::query_group::Declaration{
          .instance_name = "exporter-test",
          .engine_id = uuid::now_v7(),
      });
}

void require_event_file(const std::filesystem::path &root,
                        const std::string &extension) {
  for (const auto &entry : std::filesystem::recursive_directory_iterator(root)) {
    if (entry.is_regular_file() && entry.path().extension() == extension &&
        entry.file_size() > 0) {
      return;
    }
  }
  throw std::runtime_error("no non-empty " + extension + " event file under " +
                           root.string());
}

void test_exporter_options() {
  emit_single_event(quent::ExporterOptions::none());

  const auto root = std::filesystem::path("./events/exporter-options");
  emit_single_event(
      quent::ExporterOptions::ndjson((root / "ndjson").string()));
  require_event_file(root / "ndjson", ".ndjson");

  emit_single_event(
      quent::ExporterOptions::msgpack((root / "msgpack").string()));
  require_event_file(root / "msgpack", ".msgpack");

  emit_single_event(
      quent::ExporterOptions::postcard((root / "postcard").string()));
  require_event_file(root / "postcard", ".postcard");

  auto collector =
      quent::ExporterOptions::collector("http://127.0.0.1:7836");
  try {
    auto invalid = quent::ExporterOptions::collector("not a valid uri");
    static_cast<void>(invalid);
    throw std::runtime_error("malformed collector URI was accepted");
  } catch (const rust::Error &) {
  }
}

} // namespace

int main() {
  test_exporter_options();

  // Create instrumentation context. The context generates its own id for the
  // output subdirectory; the root engine uses an independent id here.
  auto engine_id = uuid::now_v7();
  auto ctx =
      quent::create_context(quent::ExporterOptions::ndjson("./events"));

  // Engine: init with implementation attributes and custom attributes.
  auto engine_obs = quent::engine::create_observer(*ctx);

  quent::DynamicAttributes engine_custom;
  engine_custom.string_attrs.push_back({"deployment", "test"});
  engine_custom.i64_attrs.push_back({"max_memory_mb", 4096});

  engine_obs->init(engine_id,
                   quent::engine::Init{
                       .implementation =
                           quent::engine::Implementation{
                               .name = "TestEngine",
                               .version = "1.0.0",
                               .custom_attributes = std::move(engine_custom),
                           },
                       .instance_name = "engine-0",
                   });

  // Worker: init with parent engine reference.
  auto worker_obs = quent::worker::create_observer(*ctx);
  auto worker_id = uuid::now_v7();
  worker_obs->init(worker_id, quent::worker::Init{
                                  .parent_engine_id = engine_id,
                                  .instance_name = "worker-0",
                              });

  // QueryGroup: declaration with instance name and engine id.
  auto qg_obs = quent::query_group::create_observer(*ctx);
  auto qg_id = uuid::now_v7();
  qg_obs->declaration(qg_id, quent::query_group::Declaration{
                                 .instance_name = "qg-0",
                                 .engine_id = engine_id,
                             });

  // Query FSM: create in Init state, transition through planning and executing.
  auto query = quent::query::create(*ctx, quent::query::Init{
                                              .instance_name = "select-1",
                                              .query_group_id = qg_id,
                                          });
  query->planning();
  query->executing();
  query->exit();

  // Plan: declaration with parent, edges, and optional worker.
  auto plan_obs = quent::plan::create_observer(*ctx);
  auto plan_id = uuid::now_v7();
  auto port_src = uuid::now_v7();
  auto port_tgt = uuid::now_v7();

  plan_obs->declaration(
      plan_id,
      quent::plan::Declaration{
          .parent =
              quent::plan::Parent{
                  .query_id = uuid::new_nil(),
                  .plan_id = uuid::new_nil(),
              },
          .instance_name = "physical-plan-0",
          .edges =
              {
                  quent::plan::Edges{.source = port_src, .target = port_tgt},
              },
          .worker_id = worker_id,
      });

  // Operator: declaration with plan reference, parent operators, and custom
  // attrs.
  auto op_obs = quent::operator_::create_observer(*ctx);
  auto op_id = uuid::now_v7();

  quent::DynamicAttributes op_custom;
  op_custom.string_attrs.push_back({"algo", "hash_join"});
  op_custom.f64_attrs.push_back({"selectivity", 0.75});

  op_obs->declaration(op_id, quent::operator_::Declaration{
                                 .plan_id = plan_id,
                                 .parent_operator_ids = {},
                                 .instance_name = "hash-join-0",
                                 .type_name = "HashJoin",
                                 .custom_attributes = std::move(op_custom),
                             });

  // Operator: statistics with custom attributes.
  quent::DynamicAttributes op_stats;
  op_stats.i64_attrs.push_back({"rows_processed", 10000});
  op_stats.f64_attrs.push_back({"elapsed_ms", 42.5});

  op_obs->statistics(op_id, quent::operator_::Statistics{
                                .custom_attributes = std::move(op_stats),
                            });

  // Port: declaration with operator reference.
  auto port_obs = quent::port::create_observer(*ctx);
  auto port_id = uuid::now_v7();
  port_obs->declaration(port_id, quent::port::Declaration{
                                     .operator_id = op_id,
                                     .instance_name = "output-0",
                                 });

  // Port: statistics with custom attributes.
  quent::DynamicAttributes port_stats;
  port_stats.i64_attrs.push_back({"bytes_transferred", 1048576});

  port_obs->statistics(port_id, quent::port::Statistics{
                                    .custom_attributes = std::move(port_stats),
                                });

  // Worker: exit.
  worker_obs->exit(worker_id);

  // Engine: exit.
  engine_obs->exit(engine_id);

  // At scope end the observers, handles, and context all drop; each stream
  // flushes when its last clone is released.
  return 0;
}
