// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Fixed query-engine event emitter.
//!
//! Hardcoded 7-second scenario for tests and manual UI debugging.
//! Phase boundaries land on whole-second ticks:
//!
//! - 0–1s: init (engine, 2 workers, memories, thread pools, threads, channel)
//! - 1–2s: query planning (logical plan + two physical sub-plans)
//! - 2–3s / 3–4s / 4–5s / 5–6s: ScanFilter / PartialAggregate / FinalAggregate / Limit tasks
//! - 6–7s: statistics, query exit, resource teardown
//!
//! Plan is split across workers: W0 (driver) owns FinalAggregate → Limit →
//! Output. W1's PartialAggregate tasks ship their partition to W0's
//! FinalAggregate over a channel.
//!
//! UUIDs and timestamps are plain numeric literals — grep them. Sibling:
//! `examples/simulator/` (same model, runtime entropy).

use clap::Parser;
use quent_attributes::Attribute;
use quent_exporter::{
    CollectorExporterOptions, ExporterOptions, FileSystemExporterOptions, FileSystemFormat,
};
use quent_model::{Ref, usage};
use quent_query_engine_model::{
    engine::{self, EngineImplementationAttributes},
    operator, plan, port, query_group, worker,
};
use quent_simulator_instrumentation::SimulatorContext;
use uuid::{Uuid, uuid};

// Top-level entities
const ENGINE: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
const QUERY_GROUP: Uuid = uuid!("00000000-0000-0000-0000-000000000003");
const QUERY: Uuid = uuid!("00000000-0000-0000-0000-000000000004");

// Workers
const WORKER_0: Uuid = uuid!("00000000-0000-0000-0000-000000000002");
const WORKER_1: Uuid = uuid!("00000000-0000-0000-0000-000000000021");

// Per-worker resources
const MEMORY_W0: Uuid = uuid!("00000000-0000-0000-0000-000000000022");
const MEMORY_W1: Uuid = uuid!("00000000-0000-0000-0000-000000000023");
const THREAD_POOL_W0: Uuid = uuid!("00000000-0000-0000-0000-00000000003b");
const THREAD_POOL_W1: Uuid = uuid!("00000000-0000-0000-0000-00000000003c");
const THREAD_W0_T0: Uuid = uuid!("00000000-0000-0000-0000-000000000024");
const THREAD_W0_T1: Uuid = uuid!("00000000-0000-0000-0000-000000000025");
const THREAD_W1_T0: Uuid = uuid!("00000000-0000-0000-0000-000000000026");
const THREAD_W1_T1: Uuid = uuid!("00000000-0000-0000-0000-000000000027");

// Cross-worker channel (parented to engine, used by sender tasks)
const CHANNEL_W1_W0: Uuid = uuid!("00000000-0000-0000-0000-000000000028");

// Plans
const LOGICAL_PLAN: Uuid = uuid!("00000000-0000-0000-0000-000000000005");
const PHYSICAL_PLAN_W0: Uuid = uuid!("00000000-0000-0000-0000-000000000006");
const PHYSICAL_PLAN_W1: Uuid = uuid!("00000000-0000-0000-0000-00000000002e");

// Logical operators
const LOG_SCAN: Uuid = uuid!("00000000-0000-0000-0000-000000000007");
const LOG_FILTER: Uuid = uuid!("00000000-0000-0000-0000-000000000008");
const LOG_AGGREGATE: Uuid = uuid!("00000000-0000-0000-0000-000000000009");
const LOG_LIMIT: Uuid = uuid!("00000000-0000-0000-0000-00000000000a");
const LOG_OUTPUT: Uuid = uuid!("00000000-0000-0000-0000-00000000000b");

// Physical operators (worker-0)
const PHYS_SCAN_FILTER_W0: Uuid = uuid!("00000000-0000-0000-0000-00000000000c");
const PHYS_PARTIAL_AGG_W0: Uuid = uuid!("00000000-0000-0000-0000-00000000000d");
const PHYS_FINAL_AGG: Uuid = uuid!("00000000-0000-0000-0000-00000000000e");
const PHYS_LIMIT: Uuid = uuid!("00000000-0000-0000-0000-00000000000f");
const PHYS_OUTPUT: Uuid = uuid!("00000000-0000-0000-0000-000000000010");

// Physical operators (worker-1)
const PHYS_SCAN_FILTER_W1: Uuid = uuid!("00000000-0000-0000-0000-00000000002f");
const PHYS_PARTIAL_AGG_W1: Uuid = uuid!("00000000-0000-0000-0000-000000000030");

// Logical ports
const PORT_LOG_SCAN_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000011");
const PORT_LOG_FILTER_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000012");
const PORT_LOG_FILTER_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000013");
const PORT_LOG_AGGREGATE_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000014");
const PORT_LOG_AGGREGATE_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000015");
const PORT_LOG_LIMIT_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000016");
const PORT_LOG_LIMIT_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000017");
const PORT_LOG_OUTPUT_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000018");

// Physical ports (worker-0)
const PORT_PHYS_SCAN_FILTER_W0_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000019");
const PORT_PHYS_PARTIAL_AGG_W0_IN: Uuid = uuid!("00000000-0000-0000-0000-00000000001a");
const PORT_PHYS_PARTIAL_AGG_W0_OUT: Uuid = uuid!("00000000-0000-0000-0000-00000000001b");
const PORT_PHYS_FINAL_AGG_IN: Uuid = uuid!("00000000-0000-0000-0000-00000000001c");
const PORT_PHYS_FINAL_AGG_OUT: Uuid = uuid!("00000000-0000-0000-0000-00000000001d");
const PORT_PHYS_LIMIT_IN: Uuid = uuid!("00000000-0000-0000-0000-00000000001e");
const PORT_PHYS_LIMIT_OUT: Uuid = uuid!("00000000-0000-0000-0000-00000000001f");
const PORT_PHYS_OUTPUT_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000020");

// Physical ports (worker-1)
const PORT_PHYS_SCAN_FILTER_W1_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000031");
const PORT_PHYS_PARTIAL_AGG_W1_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000032");
const PORT_PHYS_PARTIAL_AGG_W1_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000033");

// Tasks
const TASK_0: Uuid = uuid!("00000000-0000-0000-0000-000000000029");
const TASK_1: Uuid = uuid!("00000000-0000-0000-0000-00000000002a");
const TASK_2: Uuid = uuid!("00000000-0000-0000-0000-00000000002b");
const TASK_3: Uuid = uuid!("00000000-0000-0000-0000-00000000002c");
const TASK_4: Uuid = uuid!("00000000-0000-0000-0000-00000000002d");
const TASK_5: Uuid = uuid!("00000000-0000-0000-0000-000000000034");
const TASK_6: Uuid = uuid!("00000000-0000-0000-0000-000000000035");
const TASK_7: Uuid = uuid!("00000000-0000-0000-0000-000000000036");
const TASK_8: Uuid = uuid!("00000000-0000-0000-0000-000000000037");
const TASK_9: Uuid = uuid!("00000000-0000-0000-0000-000000000038");
const TASK_10: Uuid = uuid!("00000000-0000-0000-0000-000000000039");
const TASK_11: Uuid = uuid!("00000000-0000-0000-0000-00000000003a");

// ts!(N, expr) sets the next timestamp() to N, then runs expr.
macro_rules! ts {
    ($ts:expr, $($body:tt)+) => {{
        ::quent_time::set_timestamp($ts);
        { $($body)+ }
    }};
    ($ts:expr) => { ::quent_time::set_timestamp($ts) };
}

#[derive(Parser, Debug)]
#[command(name = "quent-query-engine-fixed")]
#[command(about = "Emits a fixed query-engine telemetry stream", long_about = None)]
struct Args {
    #[arg(long, default_value = "collector")]
    exporter: String,

    #[arg(
        long,
        default_value = "http://localhost:7836",
        env = "QUENT_COLLECTOR_ADDRESS"
    )]
    collector_address: String,

    #[arg(long, default_value = "events")]
    output_dir: String,
}

// Resource handles live in emit() so teardown at the bottom can call
// finalizing()/exit() on them. Bulky declaration phases are in helpers below.
fn emit(ctx: &SimulatorContext) {
    let engine_obs = ctx.engine_observer();
    let worker_obs = ctx.worker_observer();
    let group_obs = ctx.query_group_observer();
    let query_obs = ctx.query_observer();
    let mem_obs = ctx.memory_observer();
    let proc_obs = ctx.processor_observer();
    let tp_obs = ctx.thread_pool_observer();
    let ch_obs = ctx.channel_observer();

    // Init phase (0–1s).
    // All declarations and resource init at 0; all resource operating at 500ms.
    ts!(
        0,
        engine_obs.create(ENGINE).init(engine::Init {
            instance_name: Some("test-engine".into()),
            implementation: EngineImplementationAttributes {
                name: Some("Fixed".into()),
                version: Some("0.0.0".into()),
                custom_attributes: Default::default(),
            },
        })
    );
    ts!(
        0,
        worker_obs.create(WORKER_0).init(worker::Init {
            parent_engine_id: Ref::new(ENGINE),
            instance_name: "worker-0".into(),
        })
    );
    ts!(
        0,
        worker_obs.create(WORKER_1).init(worker::Init {
            parent_engine_id: Ref::new(ENGINE),
            instance_name: "worker-1".into(),
        })
    );
    let mut mem_w0 = ts!(0, mem_obs.initializing(MEMORY_W0, "memory", WORKER_0));
    let mut mem_w1 = ts!(0, mem_obs.initializing(MEMORY_W1, "memory", WORKER_1));
    ts!(
        0,
        tp_obs.thread_pool(THREAD_POOL_W0, "thread-pool", WORKER_0)
    );
    ts!(
        0,
        tp_obs.thread_pool(THREAD_POOL_W1, "thread-pool", WORKER_1)
    );
    let mut th_w0_t0 = ts!(
        0,
        proc_obs.initializing(THREAD_W0_T0, "thread-0", THREAD_POOL_W0)
    );
    let mut th_w0_t1 = ts!(
        0,
        proc_obs.initializing(THREAD_W0_T1, "thread-1", THREAD_POOL_W0)
    );
    let mut th_w1_t0 = ts!(
        0,
        proc_obs.initializing(THREAD_W1_T0, "thread-0", THREAD_POOL_W1)
    );
    let mut th_w1_t1 = ts!(
        0,
        proc_obs.initializing(THREAD_W1_T1, "thread-1", THREAD_POOL_W1)
    );
    let mut channel = ts!(
        0,
        ch_obs.initializing(
            CHANNEL_W1_W0,
            "worker-1 → worker-0",
            ENGINE,
            MEMORY_W1,
            MEMORY_W0
        )
    );

    ts!(500_000_000, mem_w0.operating(Some(1024)));
    ts!(500_000_000, mem_w1.operating(Some(1024)));
    ts!(500_000_000, th_w0_t0.operating());
    ts!(500_000_000, th_w0_t1.operating());
    ts!(500_000_000, th_w1_t0.operating());
    ts!(500_000_000, th_w1_t1.operating());
    ts!(500_000_000, channel.operating(None));

    // Query group declaration, just before the query starts.
    ts!(
        950_000_000,
        group_obs.declaration(
            QUERY_GROUP,
            query_group::Declaration {
                engine_id: ENGINE,
                instance_name: "test-group".into(),
            },
        )
    );

    // Query init + planning at 1s; executing at 2s.
    let mut query = ts!(
        1_000_000_000,
        query_obs.init(QUERY, "test-query", Ref::new(QUERY_GROUP))
    );
    ts!(1_000_000_000, query.planning());

    // Plan declarations: logical @ 1.1s; both physical plans @ 1.2s.
    declare_logical_plan(ctx);
    declare_physical_plan_w0(ctx);
    declare_physical_plan_w1(ctx);

    // Task execution (2–6s).
    ts!(2_000_000_000, query.executing());
    execute_tasks(ctx);

    // Statistics at 6.1s (op + port stats share one timestamp).
    emit_operator_statistics(ctx);
    emit_port_statistics(ctx);

    // Teardown: query exit @ 6.3s; all resource finalizing @ 6.5s; all
    // resource exit @ 6.7s; both worker exits @ 6.9s; engine exit @ 7s.
    ts!(6_300_000_000, query.exit());

    ts!(6_500_000_000, channel.finalizing());
    ts!(6_500_000_000, th_w1_t1.finalizing());
    ts!(6_500_000_000, th_w1_t0.finalizing());
    ts!(6_500_000_000, th_w0_t1.finalizing());
    ts!(6_500_000_000, th_w0_t0.finalizing());
    ts!(6_500_000_000, mem_w1.finalizing());
    ts!(6_500_000_000, mem_w0.finalizing());

    ts!(6_700_000_000, channel.exit());
    ts!(6_700_000_000, th_w1_t1.exit());
    ts!(6_700_000_000, th_w1_t0.exit());
    ts!(6_700_000_000, th_w0_t1.exit());
    ts!(6_700_000_000, th_w0_t0.exit());
    ts!(6_700_000_000, mem_w1.exit());
    ts!(6_700_000_000, mem_w0.exit());

    ts!(
        6_900_000_000,
        worker_obs.create(WORKER_1).exit(worker::Exit)
    );
    ts!(
        6_900_000_000,
        worker_obs.create(WORKER_0).exit(worker::Exit)
    );
    ts!(7_000_000_000, engine_obs.create(ENGINE).exit(engine::Exit));
}

// Logical plan: Scan → Filter → Aggregate → Limit → Output.
fn declare_logical_plan(ctx: &SimulatorContext) {
    let plan_obs = ctx.plan_observer();
    let op_obs = ctx.operator_observer();
    let port_obs = ctx.port_observer();

    let edges = vec![
        plan::Edge {
            source: Ref::new(PORT_LOG_SCAN_OUT),
            target: Ref::new(PORT_LOG_FILTER_IN),
        },
        plan::Edge {
            source: Ref::new(PORT_LOG_FILTER_OUT),
            target: Ref::new(PORT_LOG_AGGREGATE_IN),
        },
        plan::Edge {
            source: Ref::new(PORT_LOG_AGGREGATE_OUT),
            target: Ref::new(PORT_LOG_LIMIT_IN),
        },
        plan::Edge {
            source: Ref::new(PORT_LOG_LIMIT_OUT),
            target: Ref::new(PORT_LOG_OUTPUT_IN),
        },
    ];
    ts!(
        1_100_000_000,
        plan_obs.declaration(
            LOGICAL_PLAN,
            plan::Declaration {
                instance_name: "logical".into(),
                parent: plan::PlanParent {
                    query_id: Some(Ref::new(QUERY)),
                    plan_id: None,
                },
                worker_id: None,
                edges,
            },
        )
    );

    let ops: [(Uuid, &str); 5] = [
        (LOG_SCAN, "Scan"),
        (LOG_FILTER, "Filter"),
        (LOG_AGGREGATE, "Aggregate"),
        (LOG_LIMIT, "Limit"),
        (LOG_OUTPUT, "Output"),
    ];
    for (id, name) in ops {
        ts!(
            1_100_000_000,
            op_obs.create(id).declaration(operator::Declaration {
                plan_id: Ref::new(LOGICAL_PLAN),
                parent_operator_ids: vec![],
                instance_name: name.into(),
                type_name: name.into(),
                custom_attributes: Default::default(),
            })
        );
    }

    let ports: [(Uuid, Uuid, &str); 8] = [
        (PORT_LOG_SCAN_OUT, LOG_SCAN, "out"),
        (PORT_LOG_FILTER_IN, LOG_FILTER, "in"),
        (PORT_LOG_FILTER_OUT, LOG_FILTER, "out"),
        (PORT_LOG_AGGREGATE_IN, LOG_AGGREGATE, "in"),
        (PORT_LOG_AGGREGATE_OUT, LOG_AGGREGATE, "out"),
        (PORT_LOG_LIMIT_IN, LOG_LIMIT, "in"),
        (PORT_LOG_LIMIT_OUT, LOG_LIMIT, "out"),
        (PORT_LOG_OUTPUT_IN, LOG_OUTPUT, "in"),
    ];
    for (id, op_id, name) in ports {
        ts!(
            1_100_000_000,
            port_obs.create(id).declaration(port::Declaration {
                operator_id: Ref::new(op_id),
                instance_name: name.into(),
            })
        );
    }
}

// Physical plan W0 (the driver):
//   ScanFilter_W0 → PartialAggregate_W0 → FinalAggregate → Limit → Output
fn declare_physical_plan_w0(ctx: &SimulatorContext) {
    let plan_obs = ctx.plan_observer();
    let op_obs = ctx.operator_observer();
    let port_obs = ctx.port_observer();

    let edges = vec![
        plan::Edge {
            source: Ref::new(PORT_PHYS_SCAN_FILTER_W0_OUT),
            target: Ref::new(PORT_PHYS_PARTIAL_AGG_W0_IN),
        },
        plan::Edge {
            source: Ref::new(PORT_PHYS_PARTIAL_AGG_W0_OUT),
            target: Ref::new(PORT_PHYS_FINAL_AGG_IN),
        },
        plan::Edge {
            source: Ref::new(PORT_PHYS_FINAL_AGG_OUT),
            target: Ref::new(PORT_PHYS_LIMIT_IN),
        },
        plan::Edge {
            source: Ref::new(PORT_PHYS_LIMIT_OUT),
            target: Ref::new(PORT_PHYS_OUTPUT_IN),
        },
    ];
    ts!(
        1_200_000_000,
        plan_obs.declaration(
            PHYSICAL_PLAN_W0,
            plan::Declaration {
                instance_name: "physical (worker-0)".into(),
                parent: plan::PlanParent {
                    query_id: None,
                    plan_id: Some(Ref::new(LOGICAL_PLAN)),
                },
                worker_id: Some(Ref::new(WORKER_0)),
                edges,
            },
        )
    );

    let ops: [(Uuid, &str, &[Uuid]); 5] = [
        (PHYS_SCAN_FILTER_W0, "ScanFilter", &[LOG_SCAN, LOG_FILTER]),
        (PHYS_PARTIAL_AGG_W0, "PartialAggregate", &[LOG_AGGREGATE]),
        (PHYS_FINAL_AGG, "FinalAggregate", &[LOG_AGGREGATE]),
        (PHYS_LIMIT, "Limit", &[LOG_LIMIT]),
        (PHYS_OUTPUT, "Output", &[LOG_OUTPUT]),
    ];
    for (id, name, parents) in ops {
        ts!(
            1_200_000_000,
            op_obs.create(id).declaration(operator::Declaration {
                plan_id: Ref::new(PHYSICAL_PLAN_W0),
                parent_operator_ids: parents.iter().map(|p| Ref::new(*p)).collect(),
                instance_name: name.into(),
                type_name: name.into(),
                custom_attributes: Default::default(),
            })
        );
    }

    let ports: [(Uuid, Uuid, &str); 8] = [
        (PORT_PHYS_SCAN_FILTER_W0_OUT, PHYS_SCAN_FILTER_W0, "out"),
        (PORT_PHYS_PARTIAL_AGG_W0_IN, PHYS_PARTIAL_AGG_W0, "in"),
        (PORT_PHYS_PARTIAL_AGG_W0_OUT, PHYS_PARTIAL_AGG_W0, "out"),
        (PORT_PHYS_FINAL_AGG_IN, PHYS_FINAL_AGG, "in"),
        (PORT_PHYS_FINAL_AGG_OUT, PHYS_FINAL_AGG, "out"),
        (PORT_PHYS_LIMIT_IN, PHYS_LIMIT, "in"),
        (PORT_PHYS_LIMIT_OUT, PHYS_LIMIT, "out"),
        (PORT_PHYS_OUTPUT_IN, PHYS_OUTPUT, "in"),
    ];
    for (id, op_id, name) in ports {
        ts!(
            1_200_000_000,
            port_obs.create(id).declaration(port::Declaration {
                operator_id: Ref::new(op_id),
                instance_name: name.into(),
            })
        );
    }
}

// Physical plan W1 (the contributor):
//   ScanFilter_W1 → PartialAggregate_W1
// PartialAggregate_W1's output goes to W0's FinalAggregate via CHANNEL_W1_W0.
fn declare_physical_plan_w1(ctx: &SimulatorContext) {
    let plan_obs = ctx.plan_observer();
    let op_obs = ctx.operator_observer();
    let port_obs = ctx.port_observer();

    let edges = vec![plan::Edge {
        source: Ref::new(PORT_PHYS_SCAN_FILTER_W1_OUT),
        target: Ref::new(PORT_PHYS_PARTIAL_AGG_W1_IN),
    }];
    ts!(
        1_200_000_000,
        plan_obs.declaration(
            PHYSICAL_PLAN_W1,
            plan::Declaration {
                instance_name: "physical (worker-1)".into(),
                parent: plan::PlanParent {
                    query_id: None,
                    plan_id: Some(Ref::new(LOGICAL_PLAN)),
                },
                worker_id: Some(Ref::new(WORKER_1)),
                edges,
            },
        )
    );

    let ops: [(Uuid, &str, &[Uuid]); 2] = [
        (PHYS_SCAN_FILTER_W1, "ScanFilter", &[LOG_SCAN, LOG_FILTER]),
        (PHYS_PARTIAL_AGG_W1, "PartialAggregate", &[LOG_AGGREGATE]),
    ];
    for (id, name, parents) in ops {
        ts!(
            1_200_000_000,
            op_obs.create(id).declaration(operator::Declaration {
                plan_id: Ref::new(PHYSICAL_PLAN_W1),
                parent_operator_ids: parents.iter().map(|p| Ref::new(*p)).collect(),
                instance_name: name.into(),
                type_name: name.into(),
                custom_attributes: Default::default(),
            })
        );
    }

    let ports: [(Uuid, Uuid, &str); 3] = [
        (PORT_PHYS_SCAN_FILTER_W1_OUT, PHYS_SCAN_FILTER_W1, "out"),
        (PORT_PHYS_PARTIAL_AGG_W1_IN, PHYS_PARTIAL_AGG_W1, "in"),
        (PORT_PHYS_PARTIAL_AGG_W1_OUT, PHYS_PARTIAL_AGG_W1, "out"),
    ];
    for (id, op_id, name) in ports {
        ts!(
            1_200_000_000,
            port_obs.create(id).declaration(port::Declaration {
                operator_id: Ref::new(op_id),
                instance_name: name.into(),
            })
        );
    }
}

// 12 tasks, one operator per second (Scan 2s, PA 3s, FA 4s, Limit 5s).
// Each operator's two tasks run in parallel on its worker's two threads.
// Per-task: queueing + allocating at slot start, computing at +250ms,
// exit at slot end. The two PA_W1 tasks also emit a `sending` at slot+500ms.
fn execute_tasks(ctx: &SimulatorContext) {
    let task_obs = ctx.task_observer();

    #[rustfmt::skip]
    let tasks = [
        // (task, operator, t_q, t_a, t_c, t_e, thread, memory)
        // ScanFilter: 2–3s, parallel on both workers' threads.
        (TASK_0,  PHYS_SCAN_FILTER_W0, 2_000_000_000_u64, 2_000_000_000, 2_250_000_000, 3_000_000_000, THREAD_W0_T0, MEMORY_W0),
        (TASK_1,  PHYS_SCAN_FILTER_W0, 2_000_000_000,     2_000_000_000, 2_250_000_000, 3_000_000_000, THREAD_W0_T1, MEMORY_W0),
        (TASK_2,  PHYS_SCAN_FILTER_W1, 2_000_000_000,     2_000_000_000, 2_250_000_000, 3_000_000_000, THREAD_W1_T0, MEMORY_W1),
        (TASK_3,  PHYS_SCAN_FILTER_W1, 2_000_000_000,     2_000_000_000, 2_250_000_000, 3_000_000_000, THREAD_W1_T1, MEMORY_W1),
        // PartialAggregate: 3–4s, parallel on both workers' threads.
        (TASK_4,  PHYS_PARTIAL_AGG_W0, 3_000_000_000,     3_000_000_000, 3_250_000_000, 4_000_000_000, THREAD_W0_T0, MEMORY_W0),
        (TASK_5,  PHYS_PARTIAL_AGG_W0, 3_000_000_000,     3_000_000_000, 3_250_000_000, 4_000_000_000, THREAD_W0_T1, MEMORY_W0),
        (TASK_6,  PHYS_PARTIAL_AGG_W1, 3_000_000_000,     3_000_000_000, 3_250_000_000, 4_000_000_000, THREAD_W1_T0, MEMORY_W1),
        (TASK_7,  PHYS_PARTIAL_AGG_W1, 3_000_000_000,     3_000_000_000, 3_250_000_000, 4_000_000_000, THREAD_W1_T1, MEMORY_W1),
        // FinalAggregate: 4–5s, parallel on worker-0's threads.
        (TASK_8,  PHYS_FINAL_AGG,      4_000_000_000,     4_000_000_000, 4_250_000_000, 5_000_000_000, THREAD_W0_T0, MEMORY_W0),
        (TASK_9,  PHYS_FINAL_AGG,      4_000_000_000,     4_000_000_000, 4_250_000_000, 5_000_000_000, THREAD_W0_T1, MEMORY_W0),
        // Limit: 5–6s, parallel on worker-0's threads.
        (TASK_10, PHYS_LIMIT,          5_000_000_000,     5_000_000_000, 5_250_000_000, 6_000_000_000, THREAD_W0_T0, MEMORY_W0),
        (TASK_11, PHYS_LIMIT,          5_000_000_000,     5_000_000_000, 5_250_000_000, 6_000_000_000, THREAD_W0_T1, MEMORY_W0),
    ];
    for (task_id, op_id, t_q, t_a, t_c, t_e, thread, memory) in tasks {
        let mut task = ts!(t_q, task_obs.queueing(task_id, "task", op_id));
        ts!(t_a, task.allocating(Some(usage(Ref::new(thread)))));
        ts!(
            t_c,
            task.computing(
                Some(usage(Ref::new(thread))),
                Some(usage((Ref::new(memory), 256u64))),
            )
        );
        if task_id == TASK_6 || task_id == TASK_7 {
            ts!(
                t_q + 500_000_000,
                task.sending(
                    Some(usage(Ref::new(thread))),
                    Some(usage((Ref::new(CHANNEL_W1_W0), 256u64))),
                )
            );
        }
        ts!(t_e, task.exit());
    }
}

// Operator statistics — one per operator (12 total), all at 6.1s.
// The `type` attribute echoes the operator's type_name.
fn emit_operator_statistics(ctx: &SimulatorContext) {
    let op_obs = ctx.operator_observer();

    let op_stats: [(Uuid, &str); 12] = [
        (LOG_SCAN, "Scan"),
        (LOG_FILTER, "Filter"),
        (LOG_AGGREGATE, "Aggregate"),
        (LOG_LIMIT, "Limit"),
        (LOG_OUTPUT, "Output"),
        (PHYS_SCAN_FILTER_W0, "ScanFilter"),
        (PHYS_SCAN_FILTER_W1, "ScanFilter"),
        (PHYS_PARTIAL_AGG_W0, "PartialAggregate"),
        (PHYS_PARTIAL_AGG_W1, "PartialAggregate"),
        (PHYS_FINAL_AGG, "FinalAggregate"),
        (PHYS_LIMIT, "Limit"),
        (PHYS_OUTPUT, "Output"),
    ];
    for (op_id, type_name) in op_stats {
        ts!(
            6_100_000_000,
            op_obs.create(op_id).statistics(operator::Statistics {
                custom_attributes: vec![Attribute::string("type", type_name)].into(),
            })
        );
    }
}

// Port statistics — one per port (19 total), all at 6.1s (same group as op stats).
fn emit_port_statistics(ctx: &SimulatorContext) {
    let port_obs = ctx.port_observer();

    let port_stats: [Uuid; 19] = [
        PORT_LOG_SCAN_OUT,
        PORT_LOG_FILTER_IN,
        PORT_LOG_FILTER_OUT,
        PORT_LOG_AGGREGATE_IN,
        PORT_LOG_AGGREGATE_OUT,
        PORT_LOG_LIMIT_IN,
        PORT_LOG_LIMIT_OUT,
        PORT_LOG_OUTPUT_IN,
        PORT_PHYS_SCAN_FILTER_W0_OUT,
        PORT_PHYS_SCAN_FILTER_W1_OUT,
        PORT_PHYS_PARTIAL_AGG_W0_IN,
        PORT_PHYS_PARTIAL_AGG_W1_IN,
        PORT_PHYS_PARTIAL_AGG_W0_OUT,
        PORT_PHYS_PARTIAL_AGG_W1_OUT,
        PORT_PHYS_FINAL_AGG_IN,
        PORT_PHYS_FINAL_AGG_OUT,
        PORT_PHYS_LIMIT_IN,
        PORT_PHYS_LIMIT_OUT,
        PORT_PHYS_OUTPUT_IN,
    ];
    for port_id in port_stats {
        ts!(
            6_100_000_000,
            port_obs.create(port_id).statistics(port::Statistics {
                custom_attributes: Default::default(),
            })
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let exporter = match args.exporter.as_str() {
        "postcard" => Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Postcard,
            root: args.output_dir.clone().into(),
        })),
        "messagepack" => Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Msgpack,
            root: args.output_dir.clone().into(),
        })),
        "ndjson" => Some(ExporterOptions::FileSystem(FileSystemExporterOptions {
            format: FileSystemFormat::Ndjson,
            root: args.output_dir.clone().into(),
        })),
        "collector" => Some(ExporterOptions::Collector(CollectorExporterOptions {
            address: args.collector_address,
            application_id: ENGINE,
        })),
        "none" => None,
        _ => {
            return Err(format!(
                "invalid exporter '{}': must be postcard, messagepack, ndjson, collector, or none",
                args.exporter
            )
            .into());
        }
    };

    let ctx = SimulatorContext::try_new(exporter)?;
    emit(&ctx);
    drop(ctx);
    Ok(())
}
