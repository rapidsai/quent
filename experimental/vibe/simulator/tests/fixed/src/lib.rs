// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Fixed simulator event emitter.
//!
//! Hardcoded 7-second scenario for tests and manual UI debugging.
//! Phase boundaries land on whole-second ticks:
//!
//! - 0–1s: init (engine, 2 workers, memories, task executors, threads, channel)
//! - 1–2s: query planning (logical plan + two physical sub-plans)
//! - 2–3s / 3–4s / 4–5s / 5–6s: ScanFilter / PartialAggregate / FinalAggregate / Limit tasks
//! - 6–7s: statistics, query completion, resource teardown
//!
//! Plan is split across workers: W0 (driver) owns FinalAggregate → Limit →
//! Output. W1's PartialAggregate tasks ship their partition to W0's
//! FinalAggregate over a channel.
//!
//! UUIDs and timestamps are plain numeric literals — grep them. Sibling:
//! `experimental/vibe/simulator/` (same model, runtime entropy).

use quent_dynamic_attributes::DynamicAttribute;
use quent_simulator_instrumentation as instr;
use uuid::{Uuid, uuid};

type SimulatorContext = instr::Context<instr::Simulator>;

// Top-level entities
pub const ENGINE: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
pub const QUERY_GROUP: Uuid = uuid!("00000000-0000-0000-0000-000000000003");
pub const QUERY: Uuid = uuid!("00000000-0000-0000-0000-000000000004");

// Workers
pub const WORKER_0: Uuid = uuid!("00000000-0000-0000-0000-000000000002");
pub const WORKER_1: Uuid = uuid!("00000000-0000-0000-0000-000000000021");

// Per-worker resources
pub const MEMORY_W0: Uuid = uuid!("00000000-0000-0000-0000-000000000022");
pub const MEMORY_W1: Uuid = uuid!("00000000-0000-0000-0000-000000000023");
pub const TASK_EXECUTOR_W0: Uuid = uuid!("00000000-0000-0000-0000-00000000003b");
pub const TASK_EXECUTOR_W1: Uuid = uuid!("00000000-0000-0000-0000-00000000003c");
pub const THREAD_W0_T0: Uuid = uuid!("00000000-0000-0000-0000-000000000024");
pub const THREAD_W0_T1: Uuid = uuid!("00000000-0000-0000-0000-000000000025");
pub const THREAD_W1_T0: Uuid = uuid!("00000000-0000-0000-0000-000000000026");
pub const THREAD_W1_T1: Uuid = uuid!("00000000-0000-0000-0000-000000000027");

// Cross-worker network and channel (used by sender tasks)
pub const NETWORK: Uuid = uuid!("00000000-0000-0000-0000-00000000003d");
pub const CHANNEL_W1_W0: Uuid = uuid!("00000000-0000-0000-0000-000000000028");

// Plans
pub const LOGICAL_PLAN: Uuid = uuid!("00000000-0000-0000-0000-000000000005");
pub const PHYSICAL_PLAN_W0: Uuid = uuid!("00000000-0000-0000-0000-000000000006");
pub const PHYSICAL_PLAN_W1: Uuid = uuid!("00000000-0000-0000-0000-00000000002e");

// Logical operators
pub const LOG_SCAN: Uuid = uuid!("00000000-0000-0000-0000-000000000007");
pub const LOG_FILTER: Uuid = uuid!("00000000-0000-0000-0000-000000000008");
pub const LOG_AGGREGATE: Uuid = uuid!("00000000-0000-0000-0000-000000000009");
pub const LOG_LIMIT: Uuid = uuid!("00000000-0000-0000-0000-00000000000a");
pub const LOG_OUTPUT: Uuid = uuid!("00000000-0000-0000-0000-00000000000b");

// Physical operators (worker-0)
pub const PHYS_SCAN_FILTER_W0: Uuid = uuid!("00000000-0000-0000-0000-00000000000c");
pub const PHYS_PARTIAL_AGG_W0: Uuid = uuid!("00000000-0000-0000-0000-00000000000d");
pub const PHYS_FINAL_AGG: Uuid = uuid!("00000000-0000-0000-0000-00000000000e");
pub const PHYS_LIMIT: Uuid = uuid!("00000000-0000-0000-0000-00000000000f");
pub const PHYS_OUTPUT: Uuid = uuid!("00000000-0000-0000-0000-000000000010");

// Physical operators (worker-1)
pub const PHYS_SCAN_FILTER_W1: Uuid = uuid!("00000000-0000-0000-0000-00000000002f");
pub const PHYS_PARTIAL_AGG_W1: Uuid = uuid!("00000000-0000-0000-0000-000000000030");

// Logical ports
pub const PORT_LOG_SCAN_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000011");
pub const PORT_LOG_FILTER_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000012");
pub const PORT_LOG_FILTER_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000013");
pub const PORT_LOG_AGGREGATE_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000014");
pub const PORT_LOG_AGGREGATE_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000015");
pub const PORT_LOG_LIMIT_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000016");
pub const PORT_LOG_LIMIT_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000017");
pub const PORT_LOG_OUTPUT_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000018");

// Physical ports (worker-0)
pub const PORT_PHYS_SCAN_FILTER_W0_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000019");
pub const PORT_PHYS_PARTIAL_AGG_W0_IN: Uuid = uuid!("00000000-0000-0000-0000-00000000001a");
pub const PORT_PHYS_PARTIAL_AGG_W0_OUT: Uuid = uuid!("00000000-0000-0000-0000-00000000001b");
pub const PORT_PHYS_FINAL_AGG_IN: Uuid = uuid!("00000000-0000-0000-0000-00000000001c");
pub const PORT_PHYS_FINAL_AGG_OUT: Uuid = uuid!("00000000-0000-0000-0000-00000000001d");
pub const PORT_PHYS_LIMIT_IN: Uuid = uuid!("00000000-0000-0000-0000-00000000001e");
pub const PORT_PHYS_LIMIT_OUT: Uuid = uuid!("00000000-0000-0000-0000-00000000001f");
pub const PORT_PHYS_OUTPUT_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000020");

// Physical ports (worker-1)
pub const PORT_PHYS_SCAN_FILTER_W1_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000031");
pub const PORT_PHYS_PARTIAL_AGG_W1_IN: Uuid = uuid!("00000000-0000-0000-0000-000000000032");
pub const PORT_PHYS_PARTIAL_AGG_W1_OUT: Uuid = uuid!("00000000-0000-0000-0000-000000000033");

// Tasks
pub const TASK_0: Uuid = uuid!("00000000-0000-0000-0000-000000000029");
pub const TASK_1: Uuid = uuid!("00000000-0000-0000-0000-00000000002a");
pub const TASK_2: Uuid = uuid!("00000000-0000-0000-0000-00000000002b");
pub const TASK_3: Uuid = uuid!("00000000-0000-0000-0000-00000000002c");
pub const TASK_4: Uuid = uuid!("00000000-0000-0000-0000-00000000002d");
pub const TASK_5: Uuid = uuid!("00000000-0000-0000-0000-000000000034");
pub const TASK_6: Uuid = uuid!("00000000-0000-0000-0000-000000000035");
pub const TASK_7: Uuid = uuid!("00000000-0000-0000-0000-000000000036");
pub const TASK_8: Uuid = uuid!("00000000-0000-0000-0000-000000000037");
pub const TASK_9: Uuid = uuid!("00000000-0000-0000-0000-000000000038");
pub const TASK_10: Uuid = uuid!("00000000-0000-0000-0000-000000000039");
pub const TASK_11: Uuid = uuid!("00000000-0000-0000-0000-00000000003a");

// ts!(N, expr) sets the next timestamp() to N, then runs expr.
macro_rules! ts {
    ($ts:expr, $($body:tt)+) => {{
        ::quent_time::set_timestamp($ts);
        { $($body)+ }
    }};
    ($ts:expr) => { ::quent_time::set_timestamp($ts) };
}

fn entity_ref<E>(id: Uuid) -> instr::EntityRef<E> {
    instr::EntityRef::new(id, ())
}

// Resource handles live in emit() so teardown at the bottom can call
// finalizing()/exit() on them. Bulky declaration phases are in helpers below.
pub fn emit(ctx: &SimulatorContext) {
    let mut engine = ctx.observer::<instr::Engine>().handle_with_id(ENGINE);
    let mut worker_w0 = ctx.observer::<instr::Worker>().handle_with_id(WORKER_0);
    let mut worker_w1 = ctx.observer::<instr::Worker>().handle_with_id(WORKER_1);
    let mem_w0 = ctx
        .observer::<instr::HostMemory>()
        .handle_with_id(MEMORY_W0);
    let mem_w1 = ctx
        .observer::<instr::HostMemory>()
        .handle_with_id(MEMORY_W1);
    let mut executor_w0 = ctx
        .observer::<instr::TaskExecutor>()
        .handle_with_id(TASK_EXECUTOR_W0);
    let mut executor_w1 = ctx
        .observer::<instr::TaskExecutor>()
        .handle_with_id(TASK_EXECUTOR_W1);
    let th_w0_t0 = ctx
        .observer::<instr::TaskExecutorThread>()
        .handle_with_id(THREAD_W0_T0);
    let th_w0_t1 = ctx
        .observer::<instr::TaskExecutorThread>()
        .handle_with_id(THREAD_W0_T1);
    let th_w1_t0 = ctx
        .observer::<instr::TaskExecutorThread>()
        .handle_with_id(THREAD_W1_T0);
    let th_w1_t1 = ctx
        .observer::<instr::TaskExecutorThread>()
        .handle_with_id(THREAD_W1_T1);
    let mut network = ctx.observer::<instr::Network>().handle_with_id(NETWORK);
    let channel = ctx
        .observer::<instr::NetworkChannel>()
        .handle_with_id(CHANNEL_W1_W0);

    // Init phase (0–1s).
    // All declarations and resource init at 0; all resource operating at 500ms.
    ts!(
        0,
        engine
            .init(
                instr::EngineImplementationAttributes {
                    name: Some("Fixed".into()),
                    version: Some("0.0.0".into()),
                    custom_attributes: Default::default(),
                },
                Some("test-engine".into()),
            )
            .unwrap()
    );
    ts!(
        0,
        worker_w0
            .init(entity_ref(ENGINE), "worker-0".into())
            .unwrap()
    );
    ts!(
        0,
        worker_w1
            .init(entity_ref(ENGINE), "worker-1".into())
            .unwrap()
    );
    let mem_w0 = ts!(
        0,
        mem_w0.initializing("memory".into(), entity_ref(WORKER_0))
    );
    let mem_w1 = ts!(
        0,
        mem_w1.initializing("memory".into(), entity_ref(WORKER_1))
    );
    ts!(
        0,
        executor_w0
            .declaration("task-executor".into(), entity_ref(WORKER_0))
            .unwrap()
    );
    ts!(
        0,
        executor_w1
            .declaration("task-executor".into(), entity_ref(WORKER_1))
            .unwrap()
    );
    let th_w0_t0 = ts!(
        0,
        th_w0_t0.initializing("thread-0".into(), entity_ref(TASK_EXECUTOR_W0))
    );
    let th_w0_t1 = ts!(
        0,
        th_w0_t1.initializing("thread-1".into(), entity_ref(TASK_EXECUTOR_W0))
    );
    let th_w1_t0 = ts!(
        0,
        th_w1_t0.initializing("thread-0".into(), entity_ref(TASK_EXECUTOR_W1))
    );
    let th_w1_t1 = ts!(
        0,
        th_w1_t1.initializing("thread-1".into(), entity_ref(TASK_EXECUTOR_W1))
    );
    ts!(
        0,
        network
            .declaration("network".into(), entity_ref(ENGINE))
            .unwrap()
    );
    let channel = ts!(
        0,
        channel.initializing("worker-1 → worker-0".into(), entity_ref(NETWORK),)
    );

    let mem_w0 = ts!(500_000_000, mem_w0.operating());
    let mem_w1 = ts!(500_000_000, mem_w1.operating());
    let th_w0_t0 = ts!(500_000_000, th_w0_t0.operating());
    let th_w0_t1 = ts!(500_000_000, th_w0_t1.operating());
    let th_w1_t0 = ts!(500_000_000, th_w1_t0.operating());
    let th_w1_t1 = ts!(500_000_000, th_w1_t1.operating());
    let channel = ts!(500_000_000, channel.operating());

    // Query group declaration, just before the query starts.
    ts!(
        950_000_000,
        ctx.observer::<instr::QueryGroup>()
            .handle_with_id(QUERY_GROUP)
            .declaration("test-group".into(), entity_ref(ENGINE))
            .unwrap()
    );

    // Query init + planning at 1s; executing at 2s.
    let query = ctx.observer::<instr::Query>().handle_with_id(QUERY);
    let query = ts!(
        1_000_000_000,
        query.init("test-query".into(), entity_ref(QUERY_GROUP))
    );
    let query = ts!(1_000_000_000, query.planning());

    // Plan declarations: logical @ 1.1s; both physical plans @ 1.2s.
    declare_logical_plan(ctx);
    declare_physical_plan_w0(ctx);
    declare_physical_plan_w1(ctx);

    // Task execution (2–6s).
    let query = ts!(2_000_000_000, query.executing());
    execute_tasks(ctx);

    // Statistics at 6.1s (op + port stats share one timestamp).
    emit_operator_statistics(ctx);
    emit_port_statistics(ctx);

    // Teardown: query done @ 6.3s; all resource finalizing @ 6.5s; all
    // resource exit @ 6.7s; both worker exits @ 6.9s; engine exit @ 7s.
    ts!(6_300_000_000, drop(query.done()));

    let channel = ts!(6_500_000_000, channel.finalizing());
    let th_w1_t1 = ts!(6_500_000_000, th_w1_t1.finalizing());
    let th_w1_t0 = ts!(6_500_000_000, th_w1_t0.finalizing());
    let th_w0_t1 = ts!(6_500_000_000, th_w0_t1.finalizing());
    let th_w0_t0 = ts!(6_500_000_000, th_w0_t0.finalizing());
    let mem_w1 = ts!(6_500_000_000, mem_w1.finalizing());
    let mem_w0 = ts!(6_500_000_000, mem_w0.finalizing());

    ts!(6_700_000_000, drop(channel.exit()));
    ts!(6_700_000_000, drop(th_w1_t1.exit()));
    ts!(6_700_000_000, drop(th_w1_t0.exit()));
    ts!(6_700_000_000, drop(th_w0_t1.exit()));
    ts!(6_700_000_000, drop(th_w0_t0.exit()));
    ts!(6_700_000_000, drop(mem_w1.exit()));
    ts!(6_700_000_000, drop(mem_w0.exit()));

    ts!(6_900_000_000, worker_w1.exit().unwrap());
    ts!(6_900_000_000, worker_w0.exit().unwrap());
    ts!(7_000_000_000, engine.exit().unwrap());
}

// Logical plan: Scan → Filter → Aggregate → Limit → Output.
fn declare_logical_plan(ctx: &SimulatorContext) {
    let edges = vec![
        instr::Edge {
            source: entity_ref(PORT_LOG_SCAN_OUT),
            target: entity_ref(PORT_LOG_FILTER_IN),
        },
        instr::Edge {
            source: entity_ref(PORT_LOG_FILTER_OUT),
            target: entity_ref(PORT_LOG_AGGREGATE_IN),
        },
        instr::Edge {
            source: entity_ref(PORT_LOG_AGGREGATE_OUT),
            target: entity_ref(PORT_LOG_LIMIT_IN),
        },
        instr::Edge {
            source: entity_ref(PORT_LOG_LIMIT_OUT),
            target: entity_ref(PORT_LOG_OUTPUT_IN),
        },
    ];
    ts!(
        1_100_000_000,
        ctx.observer::<instr::Plan>()
            .handle_with_id(LOGICAL_PLAN)
            .declaration(
                instr::PlanParent {
                    query_id: entity_ref(QUERY),
                    plan_id: None,
                },
                "logical".into(),
                edges,
                None,
            )
            .unwrap()
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
            ctx.observer::<instr::Operator>()
                .handle_with_id(id)
                .declaration(
                    entity_ref(LOGICAL_PLAN),
                    vec![],
                    name.into(),
                    name.into(),
                    Default::default(),
                )
                .unwrap()
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
            ctx.observer::<instr::Port>()
                .handle_with_id(id)
                .declaration(entity_ref(op_id), name.into())
                .unwrap()
        );
    }
}

// Physical plan W0 (the driver):
//   ScanFilter_W0 → PartialAggregate_W0 → FinalAggregate → Limit → Output
fn declare_physical_plan_w0(ctx: &SimulatorContext) {
    let edges = vec![
        instr::Edge {
            source: entity_ref(PORT_PHYS_SCAN_FILTER_W0_OUT),
            target: entity_ref(PORT_PHYS_PARTIAL_AGG_W0_IN),
        },
        instr::Edge {
            source: entity_ref(PORT_PHYS_PARTIAL_AGG_W0_OUT),
            target: entity_ref(PORT_PHYS_FINAL_AGG_IN),
        },
        instr::Edge {
            source: entity_ref(PORT_PHYS_FINAL_AGG_OUT),
            target: entity_ref(PORT_PHYS_LIMIT_IN),
        },
        instr::Edge {
            source: entity_ref(PORT_PHYS_LIMIT_OUT),
            target: entity_ref(PORT_PHYS_OUTPUT_IN),
        },
    ];
    ts!(
        1_200_000_000,
        ctx.observer::<instr::Plan>()
            .handle_with_id(PHYSICAL_PLAN_W0)
            .declaration(
                instr::PlanParent {
                    query_id: entity_ref(QUERY),
                    plan_id: Some(entity_ref(LOGICAL_PLAN)),
                },
                "physical (worker-0)".into(),
                edges,
                Some(entity_ref(WORKER_0)),
            )
            .unwrap()
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
            ctx.observer::<instr::Operator>()
                .handle_with_id(id)
                .declaration(
                    entity_ref(PHYSICAL_PLAN_W0),
                    parents.iter().map(|p| entity_ref(*p)).collect(),
                    name.into(),
                    name.into(),
                    Default::default(),
                )
                .unwrap()
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
            ctx.observer::<instr::Port>()
                .handle_with_id(id)
                .declaration(entity_ref(op_id), name.into())
                .unwrap()
        );
    }
}

// Physical plan W1 (the contributor):
//   ScanFilter_W1 → PartialAggregate_W1
// PartialAggregate_W1's output goes to W0's FinalAggregate via CHANNEL_W1_W0.
fn declare_physical_plan_w1(ctx: &SimulatorContext) {
    let edges = vec![instr::Edge {
        source: entity_ref(PORT_PHYS_SCAN_FILTER_W1_OUT),
        target: entity_ref(PORT_PHYS_PARTIAL_AGG_W1_IN),
    }];
    ts!(
        1_200_000_000,
        ctx.observer::<instr::Plan>()
            .handle_with_id(PHYSICAL_PLAN_W1)
            .declaration(
                instr::PlanParent {
                    query_id: entity_ref(QUERY),
                    plan_id: Some(entity_ref(LOGICAL_PLAN)),
                },
                "physical (worker-1)".into(),
                edges,
                Some(entity_ref(WORKER_1)),
            )
            .unwrap()
    );

    let ops: [(Uuid, &str, &[Uuid]); 2] = [
        (PHYS_SCAN_FILTER_W1, "ScanFilter", &[LOG_SCAN, LOG_FILTER]),
        (PHYS_PARTIAL_AGG_W1, "PartialAggregate", &[LOG_AGGREGATE]),
    ];
    for (id, name, parents) in ops {
        ts!(
            1_200_000_000,
            ctx.observer::<instr::Operator>()
                .handle_with_id(id)
                .declaration(
                    entity_ref(PHYSICAL_PLAN_W1),
                    parents.iter().map(|p| entity_ref(*p)).collect(),
                    name.into(),
                    name.into(),
                    Default::default(),
                )
                .unwrap()
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
            ctx.observer::<instr::Port>()
                .handle_with_id(id)
                .declaration(entity_ref(op_id), name.into())
                .unwrap()
        );
    }
}

// 12 tasks, one operator per second (Scan 2s, PA 3s, FA 4s, Limit 5s).
// Each operator's two tasks run in parallel on its worker's two threads.
// Per-task: queueing + allocating at slot start, computing at +250ms,
// exit at slot end. The two PA_W1 tasks also emit a `sending` at slot+500ms.
fn execute_tasks(ctx: &SimulatorContext) {
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
        let task = ctx.observer::<instr::Task>().handle_with_id(task_id);
        let task = ts!(
            t_q,
            task.queueing(format!("task-{task_id}"), entity_ref(op_id))
        );
        let task = ts!(
            t_a,
            task.allocating(instr::EntityRef::new(
                thread,
                instr::TaskExecutorThreadUsage,
            ))
        );
        let task = ts!(
            t_c,
            task.computing(
                "fixed task".to_owned(),
                1_500_000_000u64,
                instr::EntityRef::new(thread, instr::TaskExecutorThreadUsage),
                Some(instr::EntityRef::new(
                    memory,
                    instr::HostMemoryUsage { bytes: 256 },
                )),
                None,
            )
        );
        if task_id == TASK_6 || task_id == TASK_7 {
            let task = ts!(
                t_q + 500_000_000,
                task.sending(
                    instr::EntityRef::new(thread, instr::TaskExecutorThreadUsage),
                    instr::EntityRef::new(
                        CHANNEL_W1_W0,
                        instr::NetworkChannelUsage { bytes: 256 },
                    ),
                )
            );
            ts!(t_e, drop(task.exit()));
        } else {
            ts!(t_e, drop(task.exit()));
        }
    }
}

// Operator statistics — one per operator (12 total), all at 6.1s.
// The `type` attribute echoes the operator's type_name.
fn emit_operator_statistics(ctx: &SimulatorContext) {
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
            ctx.observer::<instr::Operator>()
                .handle_with_id(op_id)
                .statistics(vec![DynamicAttribute::string("type", type_name)].into())
                .unwrap()
        );
    }
}

// Port statistics — one per port (19 total), all at 6.1s (same group as op stats).
fn emit_port_statistics(ctx: &SimulatorContext) {
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
            ctx.observer::<instr::Port>()
                .handle_with_id(port_id)
                .statistics(Default::default())
                .unwrap()
        );
    }
}
