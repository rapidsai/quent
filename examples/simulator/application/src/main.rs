// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::{
        Barrier,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use clap::Parser;
use petgraph::{Directed, Direction, Graph, graph::NodeIndex, visit::EdgeRef};
use quent_dynamic_attributes::{DynamicAttribute, DynamicList, DynamicStruct};
use quent_io::clap::ExporterArgs;
use quent_model::{Ref, usage};
use quent_query_engine_model::{
    engine::{self, EngineImplementationAttributes},
    operator, plan, port, query_group, worker,
};
use quent_simulator_instrumentation::{
    DEFAULT_MAX_NVTX_RANGES, NvtxCapture, NvtxCategory, NvtxLayout, NvtxPushGuard, SimulatorContext,
};
use rand::{RngExt, distr::slice::Choose, rng};
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "simulator")]
#[command(about = "Emits simulated query engine telemetry", long_about = None)]
struct Args {
    /// Number of query groups
    #[arg(long, default_value_t = 1)]
    num_query_groups: usize,

    /// Number of queries per query group
    #[arg(long, default_value_t = 1)]
    num_queries: usize,

    /// Number of tasks per operator
    #[arg(long, default_value_t = 32)]
    num_tasks: usize,

    /// Number of workers
    #[arg(long, default_value_t = 2)]
    num_workers: usize,

    /// Number of threads per worker thread pool
    #[arg(long, default_value_t = 2)]
    num_threads: usize,

    /// NVTX domains to declare (0 disables NVTX). Default is default+CCCL+libcudf.
    #[arg(long, default_value_t = 3)]
    num_nvtx_domains: usize,

    /// Named categories declared in every NVTX domain (0 = Uncategorized)
    #[arg(long, default_value_t = 5)]
    num_nvtx_categories: usize,

    /// Instant NVTX marks emitted per query
    #[arg(long, default_value_t = 2)]
    num_nvtx_marks: usize,

    /// Extra nested libcudf frames inside each scan
    #[arg(long, default_value_t = 0)]
    num_nvtx_nested_ranges: usize,

    /// Emit per-task NVTX ranges on 1 in N tasks (`0` skips them).
    #[arg(long, default_value_t = 1)]
    nvtx_task_every: usize,

    /// Maximum NVTX ranges emitted across the simulation (`0` disables ranges).
    #[arg(long, default_value_t = DEFAULT_MAX_NVTX_RANGES)]
    max_nvtx_ranges: usize,

    #[command(flatten)]
    exporter: ExporterArgs,
}

const QUERY_LEVEL_NVTX_RANGES: usize = 3;
const TARGET_NVTX_RANGES_PER_CLUSTER: usize = 32;

#[derive(Clone, Copy)]
struct NvtxWorkload {
    query_count: usize,
    worker_thread_count: usize,
}

#[derive(Clone, Copy)]
struct NvtxExecution<'a> {
    capture: &'a NvtxCapture,
    workload: NvtxWorkload,
    query_barrier: &'a Barrier,
}

impl NvtxWorkload {
    fn from_args(args: &Args) -> Self {
        Self {
            query_count: args.num_query_groups.saturating_mul(args.num_queries),
            worker_thread_count: args.num_workers.saturating_mul(args.num_threads),
        }
    }

    fn detailed_ranges_per_thread(self, layout: NvtxLayout) -> usize {
        let query_thread_count = self.query_count.saturating_mul(self.worker_thread_count);
        if layout.num_domains == 0 || query_thread_count == 0 {
            return 0;
        }
        let query_ranges = self.query_count.saturating_mul(QUERY_LEVEL_NVTX_RANGES);
        let envelope_ranges = query_thread_count.saturating_mul(layout.num_domains);
        layout
            .max_ranges
            .saturating_sub(query_ranges.saturating_add(envelope_ranges))
            / query_thread_count
    }
}

fn nvtx_push_budgeted<'a>(
    nvtx: &'a NvtxCapture,
    range_budget: &mut usize,
    domain: u64,
    thread_id: u32,
    message: &str,
    category: u32,
) -> NvtxPushGuard<'a> {
    let enabled = *range_budget != 0;
    *range_budget = range_budget.saturating_sub(1);
    nvtx.push_if(enabled, domain, thread_id, message, category)
}

fn initialize_tracing() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();
}

fn log_resource_links(engine_id: Uuid, query_id: Uuid, resource_id: Uuid, resource_name: &str) {
    debug!("\tResource: {resource_name}
\t\tTimeline: http://localhost:8080/analyzer/engine/{engine_id}/query/{query_id}/resource/{resource_id}/timeline?num_bins=16&start=0&end=4"
    );
}

fn log_resource_group_links(
    engine_id: Uuid,
    query_id: Uuid,
    resource_group_id: Uuid,
    resource_group_name: &str,
) {
    debug!("\tResource Group: {resource_group_name}
\t\tTimeline: http://localhost:8080/analyzer/engine/{engine_id}/query/{query_id}/resource_group/{resource_group_id}/timeline?num_bins=16&start=0&end=4"
    );
}

fn sleep_short() {
    std::thread::sleep(Duration::from_micros(1));
}

fn sleep_long() {
    std::thread::sleep(Duration::from_micros(25));
}

fn sleep_sometimes_really_long() {
    // make 1% tasks incredibly slow
    std::thread::sleep(Duration::from_micros(if rng().random_ratio(1, 100) {
        50000
    } else {
        25
    }));
}

struct Operator<T: Debug> {
    id: Uuid,
    parents: Vec<Uuid>,
    kind: T,
    tasks_processed: AtomicU64,
}

impl<T> Operator<T>
where
    T: Debug,
{
    fn name(&self) -> String {
        format!("{:?}", self.kind)
    }

    fn new(kind: T, parents: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::now_v7(),
            parents,
            kind,
            tasks_processed: AtomicU64::new(0),
        }
    }
}

impl<T> Display for Operator<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug)]
struct Port {
    id: Uuid,
    name: &'static str,
    num_bytes: AtomicU64,
    num_rows: AtomicU64,
}

#[derive(Debug)]
struct Edge {
    source: Port,
    target: Port,
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Edge {
    fn new(source: &'static str, target: &'static str) -> Edge {
        Edge {
            source: Port {
                id: Uuid::now_v7(),
                name: source,
                num_bytes: AtomicU64::new(0),
                num_rows: AtomicU64::new(0),
            },
            target: Port {
                id: Uuid::now_v7(),
                name: target,
                num_bytes: AtomicU64::new(0),
                num_rows: AtomicU64::new(0),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Logical {
    Scan,
    Project,
    Join,
    Sort,
    Limit,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Physical {
    FileSystemScan,
    JoinPartition,
    JoinLocal,
    Sort,
    Limit,
    Output,
}

fn nvtx_pipeline_op(kind: Physical) -> &'static str {
    match kind {
        Physical::FileSystemScan => "GPU_SCAN",
        Physical::JoinPartition => "PARTITION",
        Physical::JoinLocal => "HASH_GROUP_BY",
        Physical::Sort => "ORDER_BY",
        Physical::Limit => "MERGE_SORT",
        Physical::Output => "RESULT_COLLECTOR",
    }
}

fn nvtx_execute_name(kind: Physical) -> &'static str {
    match kind {
        Physical::FileSystemScan => "read_parquet",
        Physical::JoinPartition => "sirius_physical_partition::execute",
        Physical::JoinLocal => "sirius_physical_grouped_aggregate::execute",
        Physical::Sort => "sirius_physical_order::execute",
        Physical::Limit => "sirius_physical_merge_sort::execute",
        Physical::Output => "sirius_physical_result_collector::execute",
    }
}

fn nvtx_category_for_operator(kind: Physical) -> NvtxCategory {
    match kind {
        Physical::FileSystemScan | Physical::Output => NvtxCategory::Io,
        Physical::JoinPartition => NvtxCategory::Memory,
        Physical::JoinLocal | Physical::Sort => NvtxCategory::Compute,
        Physical::Limit => NvtxCategory::Internal,
    }
}

fn nvtx_libcudf_scan(nvtx: &NvtxCapture, thread_id: u32, range_budget: &mut usize) {
    let Some(domain) = nvtx.try_domain(2) else {
        return;
    };
    let _parquet = nvtx_push_budgeted(
        nvtx,
        range_budget,
        domain,
        thread_id,
        "read_parquet",
        nvtx.category_id(NvtxCategory::Io),
    );
    {
        let _chunk = nvtx_push_budgeted(
            nvtx,
            range_budget,
            domain,
            thread_id,
            "read_chunk_internal",
            nvtx.category_id(NvtxCategory::Internal),
        );
        {
            let _decode = nvtx_push_budgeted(
                nvtx,
                range_budget,
                domain,
                thread_id,
                "decode_page_data",
                nvtx.category_id(NvtxCategory::Compute),
            );
            let nested_count = nvtx.layout().num_nested_ranges.min(*range_budget);
            *range_budget -= nested_count;
            let _extra = nvtx.push_nested(thread_id, nested_count);
            sleep_short();
        }
    }
    {
        let _copy = nvtx_push_budgeted(
            nvtx,
            range_budget,
            domain,
            thread_id,
            "copy_if",
            nvtx.category_id(NvtxCategory::Memory),
        );
        sleep_short();
    }
    {
        let _finalize = nvtx_push_budgeted(
            nvtx,
            range_budget,
            domain,
            thread_id,
            "finalize_output",
            nvtx.category_id(NvtxCategory::Internal),
        );
        sleep_short();
    }
}

fn nvtx_cccl_kernel(nvtx: &NvtxCapture, thread_id: u32, index: usize, range_budget: &mut usize) {
    let Some(domain) = nvtx.try_domain(1) else {
        return;
    };
    let _kernel = nvtx_push_budgeted(
        nvtx,
        range_budget,
        domain,
        thread_id,
        NvtxCapture::cccl_kernel_name(index),
        nvtx.category_id(NvtxCategory::Compute),
    );
    sleep_short();
}

struct Plan<T>
where
    T: Debug,
{
    id: Uuid,
    name: String,
    query_id: Uuid,
    parent_plan_id: Option<Uuid>,
    dag: Graph<Operator<T>, Edge, Directed>,
    execute: bool,
}

impl<T: Debug> Plan<T> {
    pub fn declare(&self, context: &SimulatorContext, worker_id: Option<Uuid>) {
        let plan_obs = context.plan_observer();
        let operator_obs = context.operator_observer();
        let port_obs = context.port_observer();

        plan_obs.declaration(
            self.id,
            plan::Declaration {
                instance_name: self.name.clone(),
                parent: match self.parent_plan_id {
                    None => plan::PlanParent {
                        query_id: Some(Ref::new(self.query_id)),
                        plan_id: None,
                    },
                    Some(parent_id) => plan::PlanParent {
                        query_id: None,
                        plan_id: Some(Ref::new(parent_id)),
                    },
                },
                worker_id: worker_id.map(Ref::new),
                edges: self
                    .dag
                    .edge_references()
                    .map(|edge| plan::Edge {
                        source: Ref::new(edge.weight().source.id),
                        target: Ref::new(edge.weight().target.id),
                    })
                    .collect(),
            },
        );

        // Declare all operators
        for node_idx in self.dag.node_indices() {
            let op = &self.dag[node_idx];
            let op_handle = operator_obs.create(op.id);
            op_handle.declaration(operator::Declaration {
                plan_id: Ref::new(self.id),
                parent_operator_ids: op.parents.iter().map(|&id| Ref::new(id)).collect(),
                instance_name: format!("{}-{node_idx:?}", op.name()),
                type_name: op.name(),
                custom_attributes: Default::default(),
            });

            // Declare operator ports
            for (id, event) in self
                .dag
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .map(|edge| {
                    (
                        edge.weight().target.id,
                        port::Declaration {
                            operator_id: Ref::new(op.id),
                            instance_name: edge.weight().target.name.to_string(),
                        },
                    )
                })
                .chain(
                    self.dag
                        .edges_directed(node_idx, petgraph::Direction::Outgoing)
                        .map(|edge| {
                            (
                                edge.weight().source.id,
                                port::Declaration {
                                    operator_id: Ref::new(op.id),
                                    instance_name: edge.weight().source.name.to_string(),
                                },
                            )
                        }),
                )
            {
                let port_handle = port_obs.create(id);
                port_handle.declaration(event);
            }
        }
    }
}

// Create the following logical plan:
// Scan -> Project \
//                  -> Join -> Sort -> Limit -> Output
// Scan -> Project /
fn make_logical_plan(query_id: Uuid, name: String) -> Plan<Logical> {
    // Add a scan --> project branch and return the (project, project output port) Uuids.
    fn add_scan_project_branch(plan: &mut Graph<Operator<Logical>, Edge, Directed>) -> NodeIndex {
        let scan = plan.add_node(Operator::new(Logical::Scan, vec![]));
        let project = plan.add_node(Operator::new(Logical::Project, vec![]));
        plan.add_edge(scan, project, Edge::new("out", "in"));

        project
    }

    let mut dag = Graph::new();

    let project_a = add_scan_project_branch(&mut dag);
    let project_b = add_scan_project_branch(&mut dag);

    let join = dag.add_node(Operator::new(Logical::Join, vec![]));
    dag.add_edge(project_a, join, Edge::new("out", "left"));
    dag.add_edge(project_b, join, Edge::new("out", "right"));

    let sort = dag.add_node(Operator::new(Logical::Sort, vec![]));
    dag.add_edge(join, sort, Edge::new("out", "in"));

    let limit = dag.add_node(Operator::new(Logical::Limit, vec![]));
    dag.add_edge(sort, limit, Edge::new("out", "in"));

    let output = dag.add_node(Operator::new(Logical::Output, vec![]));
    dag.add_edge(limit, output, Edge::new("out", "in"));

    Plan {
        id: Uuid::now_v7(),
        name,
        query_id,
        parent_plan_id: None,
        dag,
        execute: false,
    }
}

fn simulate_planning(logical: &Plan<Logical>) -> Plan<Physical> {
    // Find the output node
    let output = logical
        .dag
        .node_indices()
        .collect::<Vec<_>>()
        .into_iter()
        .find(|n| logical.dag[*n].kind == Logical::Output)
        .unwrap();

    // Build a physical plan
    let mut physical = Plan {
        id: Uuid::now_v7(),
        name: "physical".into(),
        query_id: logical.query_id,
        parent_plan_id: Some(logical.id),
        dag: Graph::new(),
        execute: true,
    };

    lower_logical(logical, &mut physical, output, None);

    physical
}

fn lower_logical(
    logical: &Plan<Logical>,
    physical: &mut Plan<Physical>,
    logical_current_idx: NodeIndex,
    physical_target_idx_port: Option<(NodeIndex, &'static str)>,
) {
    let current_logical_op = &logical.dag[logical_current_idx];

    match current_logical_op.kind {
        Logical::Scan => {
            unimplemented!("this shouldn't happen in this simulator, yet")
        }
        Logical::Project => {
            // Check whether this project has an incoming scan source to simulate predicate pushdown
            if let Some(scan_edge) = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .find(|edge| logical.dag[edge.source()].kind == Logical::Scan)
            {
                let scan_op = &logical.dag[scan_edge.source()];
                let source = physical.dag.add_node(Operator::new(
                    Physical::FileSystemScan,
                    vec![current_logical_op.id, scan_op.id],
                ));
                if let Some((target_node, target_port)) = physical_target_idx_port {
                    physical
                        .dag
                        .add_edge(source, target_node, Edge::new(target_port, "in"));
                }
            } else {
                unimplemented!("this shouldn't happen in this simulator, yet");
            }
        }
        Logical::Join => {
            // split up in a partition stage and join stage
            let partition = physical.dag.add_node(Operator::new(
                Physical::JoinPartition,
                vec![current_logical_op.id],
            ));
            let local = physical.dag.add_node(Operator::new(
                Physical::JoinLocal,
                vec![current_logical_op.id],
            ));
            physical
                .dag
                .add_edge(partition, local, Edge::new("build_out", "build_in"));
            physical
                .dag
                .add_edge(partition, local, Edge::new("probe_out", "probe_in"));

            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(local, target_node, Edge::new("out", target_port));
            }

            // Recurse up both branches
            for input_edge in logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
            {
                lower_logical(
                    logical,
                    physical,
                    input_edge.source(),
                    Some((partition, input_edge.weight().target.name)),
                );
            }
        }
        Logical::Sort => {
            let sort = physical
                .dag
                .add_node(Operator::new(Physical::Sort, vec![current_logical_op.id]));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(sort, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                logical,
                physical,
                input_edge.source(),
                Some((sort, input_edge.weight().target.name)),
            );
        }
        Logical::Limit => {
            let limit = physical
                .dag
                .add_node(Operator::new(Physical::Limit, vec![current_logical_op.id]));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(limit, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                logical,
                physical,
                input_edge.source(),
                Some((limit, input_edge.weight().target.name)),
            );
        }
        Logical::Output => {
            let output = physical
                .dag
                .add_node(Operator::new(Physical::Output, vec![current_logical_op.id]));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(output, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                logical,
                physical,
                input_edge.source(),
                Some((output, input_edge.weight().target.name)),
            );
        }
    }
}

struct Worker {
    id: Uuid,
    memory: Uuid,
    filesystem: Uuid,
    fs_to_mem: Uuid,
    mem_to_fs: Uuid,
    thread_pool: Uuid,
    threads: Vec<Uuid>,
    nvtx_thread_ids: Vec<u32>,
    // Resource handles — kept alive until shut_down().
    memory_handles: Vec<quent_stdlib::memory::MemoryHandle>,
    channel_handles: Vec<quent_stdlib::channel::ChannelHandle>,
    processor_handles: Vec<quent_stdlib::processor::ProcessorHandle>,
}

impl Worker {
    fn new(
        id: Uuid,
        name: String,
        context: &SimulatorContext,
        nvtx: &NvtxCapture,
        parent_engine_id: Uuid,
        num_threads: usize,
    ) -> Self {
        let worker_obs = context.worker_observer();

        info!("Spawning worker {name}");
        let worker_handle = worker_obs.create(id);
        worker_handle.init(worker::Init {
            parent_engine_id: Ref::new(parent_engine_id),
            instance_name: name.clone(),
        });

        let mut memory_handles = Vec::new();
        let mut channel_handles = Vec::new();
        let mut processor_handles = Vec::new();

        let mem_obs = context.memory_observer();
        let ch_obs = context.channel_observer();
        let proc_obs = context.processor_observer();

        // Filesystem
        let filesystem = Uuid::now_v7();
        let mut fs_handle = mem_obs.initializing(filesystem, "Filesystem", id);
        fs_handle.operating(Some(0));
        memory_handles.push(fs_handle);

        // Memory pool
        let memory = Uuid::now_v7();
        let mut mem_handle = mem_obs.initializing(memory, "Memory", id);
        mem_handle.operating(Some(0));
        memory_handles.push(mem_handle);

        // Filesystem -> Memory channel
        let fs_to_mem = Uuid::now_v7();
        let mut fs_to_mem_handle =
            ch_obs.initializing(fs_to_mem, "Filesystem -> Memory", id, filesystem, memory);
        fs_to_mem_handle.operating(None);
        channel_handles.push(fs_to_mem_handle);

        // Memory -> Filesystem channel
        let mem_to_fs = Uuid::now_v7();
        let mut mem_to_fs_handle =
            ch_obs.initializing(mem_to_fs, "Memory -> Filesystem", id, memory, filesystem);
        mem_to_fs_handle.operating(None);
        channel_handles.push(mem_to_fs_handle);

        // Thread pool
        let tp_obs = context.thread_pool_observer();
        let thread_pool = Uuid::now_v7();
        tp_obs.thread_pool(thread_pool, "Thread Pool", id);

        let mut threads = Vec::new();
        let mut nvtx_thread_ids = Vec::new();
        for index in 0..num_threads {
            let thread_id = Uuid::now_v7();
            let mut thread_handle =
                proc_obs.initializing(thread_id, &format!("Thread {index}"), thread_pool);
            threads.push(thread_id);
            nvtx_thread_ids.push(nvtx.name_thread(&format!("{name} Thread {index}")));
            thread_handle.operating();
            processor_handles.push(thread_handle);
        }

        Self {
            id,
            memory,
            filesystem,
            fs_to_mem,
            mem_to_fs,
            thread_pool,
            threads,
            nvtx_thread_ids,
            memory_handles,
            channel_handles,
            processor_handles,
        }
    }

    fn nvtx_thread_id(&self, thread: Uuid) -> u32 {
        let index = self
            .threads
            .iter()
            .position(|id| *id == thread)
            .expect("thread belongs to this worker");
        self.nvtx_thread_ids[index]
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_physical_operator_task(
        &self,
        context: &SimulatorContext,
        nvtx: &NvtxCapture,
        index: usize,
        op_id: usize,
        engine: &Engine,
        operator: &Operator<Physical>,
        thread: Uuid,
        range_budget: &mut usize,
    ) {
        let thread_ref = Ref::new(thread);
        let mem_ref = Ref::new(self.memory);
        let nvtx_thread_id = self.nvtx_thread_id(thread);
        let category = nvtx.category_id(nvtx_category_for_operator(operator.kind));
        let emit_nvtx = *range_budget != 0;
        let _op = emit_nvtx.then(|| {
            nvtx_push_budgeted(
                nvtx,
                range_budget,
                nvtx.domain_at(0),
                nvtx_thread_id,
                &format!(
                    "Pipeline 0: {} (id={op_id})",
                    nvtx_pipeline_op(operator.kind)
                ),
                nvtx.category_id(NvtxCategory::Internal),
            )
        });
        let _execute = emit_nvtx.then(|| {
            nvtx_push_budgeted(
                nvtx,
                range_budget,
                nvtx.domain_at(0),
                nvtx_thread_id,
                nvtx_execute_name(operator.kind),
                category,
            )
        });

        // Create task — emits entry -> Queueing
        let task_obs = context.task_observer();
        let mut task = task_obs.queueing(Uuid::now_v7(), &format!("task-{index}"), operator.id);

        sleep_long();
        let (spill, load, send) = match operator.kind {
            Physical::FileSystemScan => (false, rng().random_bool(0.5), false),
            Physical::JoinPartition => (false, rng().random_bool(0.5), true),
            Physical::JoinLocal => (true, rng().random_bool(0.5), false),
            Physical::Sort => (false, rng().random_bool(0.5), false),
            Physical::Limit => (false, rng().random_bool(0.5), false),
            Physical::Output => (false, rng().random_bool(0.5), false),
        };

        let num_bytes = rng().random_range(0..1024) * 1024 * 1024;

        if emit_nvtx && operator.kind == Physical::FileSystemScan {
            nvtx_libcudf_scan(nvtx, nvtx_thread_id, range_budget);
        }

        {
            task.allocating(Some(usage(thread_ref)));
            sleep_short();
        }

        if spill {
            task.spilling(
                Some(usage(thread_ref)),
                Some(usage((Ref::new(self.mem_to_fs), num_bytes))),
            );
            sleep_sometimes_really_long();
            task.allocating(Some(usage(thread_ref)));
            sleep_short();
        }

        if load {
            if emit_nvtx && operator.kind != Physical::FileSystemScan {
                nvtx_libcudf_scan(nvtx, nvtx_thread_id, range_budget);
            }
            task.loading(
                Some(usage(thread_ref)),
                Some(usage((Ref::new(self.fs_to_mem), num_bytes))),
                Some(usage((mem_ref, rng().random_range(0..4) * num_bytes))),
            );
            sleep_sometimes_really_long();
        }

        {
            if emit_nvtx {
                nvtx_cccl_kernel(
                    nvtx,
                    nvtx_thread_id,
                    index.wrapping_add(op_id),
                    range_budget,
                );
                if operator.kind == Physical::JoinLocal
                    && let Some(domain) = nvtx.try_domain(2)
                {
                    let _binop = nvtx_push_budgeted(
                        nvtx,
                        range_budget,
                        domain,
                        nvtx_thread_id,
                        "binary_operation",
                        nvtx.category_id(NvtxCategory::Compute),
                    );
                    sleep_short();
                }
            }
            task.computing(
                /* instance_name */ "",
                /* input_bytes */ num_bytes,
                Some(usage(thread_ref)),
                Some(usage((mem_ref, rng().random_range(0..4) * num_bytes))),
            );
        }

        if send {
            let other_workers = engine.workers.keys().filter(|w| **w != self.id);

            for other in other_workers {
                let link = *engine.network_links.get(&(self.id, *other)).unwrap();

                task.sending(
                    Some(usage(thread_ref)),
                    Some(usage((Ref::new(link), num_bytes))),
                );
                sleep_long();
            }
        }

        task.exit();
    }

    fn execute_logical_plan(
        &self,
        context: &SimulatorContext,
        nvtx_execution: NvtxExecution<'_>,
        engine: &Engine,
        l_plan: &Plan<Logical>,
        num_tasks: usize,
    ) {
        let NvtxExecution {
            capture: nvtx,
            workload: nvtx_workload,
            query_barrier: nvtx_query_barrier,
        } = nvtx_execution;
        let physical_plan = simulate_planning(l_plan);
        physical_plan.declare(context, Some(self.id));

        // Log analyzer debug links:
        log_resource_links(engine.id, physical_plan.query_id, self.memory, "Memory");
        log_resource_links(
            engine.id,
            physical_plan.query_id,
            self.filesystem,
            "Filesystem",
        );
        log_resource_links(
            engine.id,
            physical_plan.query_id,
            self.fs_to_mem,
            "Filesystem -> Memory",
        );
        log_resource_links(
            engine.id,
            physical_plan.query_id,
            self.mem_to_fs,
            "Memory -> Filesystem",
        );
        log_resource_group_links(
            engine.id,
            physical_plan.query_id,
            self.thread_pool,
            "Thread Pool",
        );
        for (index, thread_id) in self.threads.iter().enumerate() {
            log_resource_links(
                engine.id,
                physical_plan.query_id,
                *thread_id,
                format!("Thread {index}").as_str(),
            );
        }

        let nodes = petgraph::algo::toposort(&physical_plan.dag, None).unwrap();
        info!(
            "Topological order: {:?}",
            nodes
                .iter()
                .map(|node| format!("{:?}: {:?}", node, physical_plan.dag[*node].kind))
                .collect::<Vec<_>>()
        );

        if physical_plan.execute {
            // On each thread, run a bunch of tasks for each operator.
            let tasks_per_thread_per_op = num_tasks / self.threads.len();
            let detailed_ranges_per_thread =
                nvtx_workload.detailed_ranges_per_thread(nvtx.layout());
            let plan = &physical_plan;
            let nodes = &nodes;
            std::thread::scope(|s| {
                for (thread_index, thread_id) in self.threads.iter().enumerate() {
                    let nvtx_thread_id = self.nvtx_thread_id(*thread_id);
                    s.spawn({
                        let thread_id = *thread_id;
                        move || {
                            let _query_envelopes = (0..nvtx.layout().num_domains)
                                .map(|domain_index| {
                                    nvtx.push(
                                        nvtx.domain_at(domain_index),
                                        nvtx_thread_id,
                                        "query thread execution",
                                        nvtx.category_id(NvtxCategory::Internal),
                                    )
                                })
                                .collect::<Vec<_>>();
                            nvtx_query_barrier.wait();
                            let pipeline = nodes
                                .iter()
                                .map(|node_idx| nvtx_pipeline_op(plan.dag[*node_idx].kind))
                                .collect::<Vec<_>>()
                                .join(" -> ");
                            let requested_clusters = if detailed_ranges_per_thread == 0 {
                                0
                            } else {
                                detailed_ranges_per_thread
                                    .div_ceil(TARGET_NVTX_RANGES_PER_CLUSTER)
                                    .max(2)
                                    .min(detailed_ranges_per_thread)
                            };
                            let sampled_offsets = nvtx
                                .sampled_task_offsets(tasks_per_thread_per_op, requested_clusters);
                            let mut next_cluster = 0;
                            let task_range = thread_index * tasks_per_thread_per_op
                                ..(thread_index + 1) * tasks_per_thread_per_op;
                            for (task_offset, task_index) in task_range.enumerate() {
                                let mut range_budget =
                                    if sampled_offsets.get(next_cluster) == Some(&task_offset) {
                                        let cluster_count = sampled_offsets.len();
                                        let budget = detailed_ranges_per_thread / cluster_count
                                            + usize::from(
                                                next_cluster
                                                    < detailed_ranges_per_thread % cluster_count,
                                            );
                                        next_cluster += 1;
                                        budget
                                    } else {
                                        0
                                    };
                                let _pipeline_task = (range_budget != 0).then(|| {
                                    nvtx_push_budgeted(
                                        nvtx,
                                        &mut range_budget,
                                        nvtx.domain_at(0),
                                        nvtx_thread_id,
                                        &format!("Pipeline 0 Task {task_index} [{pipeline}]"),
                                        nvtx.category_id(NvtxCategory::Internal),
                                    )
                                });
                                for (op_id, node_idx) in nodes.iter().enumerate() {
                                    let op = &plan.dag[*node_idx];
                                    self.execute_physical_operator_task(
                                        context,
                                        nvtx,
                                        task_index,
                                        op_id,
                                        engine,
                                        op,
                                        thread_id,
                                        &mut range_budget,
                                    );
                                    op.tasks_processed.fetch_add(1, Ordering::Relaxed);
                                    let edges =
                                        plan.dag.edges_directed(*node_idx, Direction::Outgoing);
                                    for edge in edges {
                                        edge.weight().source.num_bytes.fetch_add(
                                            rng().random_range(1024..1024 * 1024),
                                            Ordering::Relaxed,
                                        );
                                        edge.weight().source.num_rows.fetch_add(
                                            rng().random_range(16..1024),
                                            Ordering::Relaxed,
                                        );
                                        edge.weight().target.num_bytes.fetch_add(
                                            rng().random_range(1024..128 * 1024),
                                            Ordering::Relaxed,
                                        );
                                        edge.weight().target.num_rows.fetch_add(
                                            rng().random_range(16..1024),
                                            Ordering::Relaxed,
                                        );
                                    }
                                }
                            }
                            nvtx_query_barrier.wait();
                        }
                    });
                }
            });
        }

        // Set some stats
        macro_rules! attr {
            (u64 $name:expr, $range:expr) => { DynamicAttribute::u64($name, rng().random_range($range)) };
            (u32 $name:expr, $val:expr) => { DynamicAttribute::u32($name, $val) };
            (f64 $name:expr, $range:expr) => { DynamicAttribute::f64($name, rng().random_range($range)) };
            (str $name:expr, $val:expr) => { DynamicAttribute::string($name, $val) };
            (pick $name:expr, $($choice:expr),+) => {
                DynamicAttribute::string($name, *rng().sample(Choose::new(&[$($choice),+]).unwrap()))
            };
        }

        let op_obs = context.operator_observer();
        let port_obs = context.port_observer();
        for node_idx in nodes.iter() {
            let op = &physical_plan.dag[*node_idx];
            let tasks_processed = op.tasks_processed.load(Ordering::Relaxed);

            // Common metrics for all operators
            let mut attributes = vec![
                DynamicAttribute::u64("tasks_processed", tasks_processed),
                attr!(u64 "wall_time_ns",       100_000..5_000_000_000),
                attr!(u64 "cpu_time_ns",        50_000..4_000_000_000),
                attr!(u64 "peak_memory_bytes",  1024..512 * 1024 * 1024),
                attr!(u64 "output_rows",        0..10_000_000),
                attr!(u64 "output_bytes",       0..2u64 * 1024 * 1024 * 1024),
                attr!(u64 "input_rows",         0..50_000_000),
                attr!(u64 "input_bytes",        0..4u64 * 1024 * 1024 * 1024),
                attr!(u64 "num_batches",        1..2048),
                attr!(f64 "avg_batch_rows",     64.0..65536.0),
            ];

            match op.kind {
                Physical::FileSystemScan => {
                    let num_files: u64 = rng().random_range(1..256);
                    attributes.extend([
                        attr!(str "file_name",              "/dev/null"),
                        DynamicAttribute::u64("files_scanned", num_files),
                        attr!(u64 "bytes_read",             1024..8u64 * 1024 * 1024 * 1024),
                        attr!(u64 "row_groups_read",        1..1024),
                        attr!(u64 "row_groups_skipped",     0..512),
                        attr!(u64 "pages_read",             1..8192),
                        attr!(u64 "pages_decompressed",     1..8192),
                        attr!(u64 "io_wait_ns",             10_000..2_000_000_000),
                        attr!(f64 "io_throughput_mbs",      50.0..6000.0),
                        attr!(u64 "decompress_time_ns",     10_000..500_000_000),
                        attr!(u64 "predicate_filter_time_ns", 0..100_000_000),
                        attr!(f64 "predicate_selectivity",  0.001..1.0),
                        attr!(u64 "null_count",             0..100_000),
                        attr!(u64 "columns_projected",      1..64),
                        // Per-file byte counts
                        DynamicAttribute::list(
                            "per_file_bytes_read",
                            DynamicList::U64(
                                (0..num_files)
                                    .map(|_| rng().random_range(1024..1024 * 1024 * 1024))
                                    .collect(),
                            ),
                        ),
                        // Column projection info
                        DynamicAttribute::list(
                            "projected_column_names",
                            DynamicList::String(
                                [
                                    "id", "name", "ts", "amount", "region", "status", "category",
                                    "score",
                                ]
                                .iter()
                                .take(rng().random_range(1..8))
                                .map(|s| s.to_string())
                                .collect(),
                            ),
                        ),
                    ]);
                }
                Physical::JoinPartition => {
                    let num_partitions: u64 = rng().random_range(2..256);
                    attributes.extend([
                        attr!(u64  "average_partition_size_bytes", 1..1024 * 1024 * 1024),
                        attr!(pick "join_strategy",          "broadcast", "hash partition"),
                        DynamicAttribute::u64("num_partitions", num_partitions),
                        attr!(u64  "partition_time_ns",      100_000..1_000_000_000),
                        attr!(u64  "hash_time_ns",           50_000..500_000_000),
                        attr!(f64  "partition_skew",         0.0..5.0),
                        attr!(u64  "max_partition_rows",     100..1_000_000),
                        attr!(u64  "min_partition_rows",     0..10_000),
                        attr!(u64  "build_side_bytes",       1024..2u64 * 1024 * 1024 * 1024),
                        attr!(u64  "probe_side_bytes",       1024..4u64 * 1024 * 1024 * 1024),
                        attr!(u64  "network_bytes_sent",     0..2u64 * 1024 * 1024 * 1024),
                        attr!(u64  "network_time_ns",        0..2_000_000_000),
                        // Row count per partition
                        DynamicAttribute::list(
                            "partition_row_counts",
                            DynamicList::U64(
                                (0..num_partitions)
                                    .map(|_| rng().random_range(0..1_000_000))
                                    .collect(),
                            ),
                        ),
                    ]);
                }
                Physical::JoinLocal => attributes.extend([
                    attr!(u64 "hash_table_size_bytes",   1024..2u64 * 1024 * 1024 * 1024),
                    attr!(u64 "hash_table_entries",      100..50_000_000),
                    attr!(u64 "build_time_ns",           100_000..2_000_000_000),
                    attr!(u64 "probe_time_ns",           100_000..3_000_000_000),
                    attr!(u64 "build_rows",              100..10_000_000),
                    attr!(u64 "probe_rows",              100..50_000_000),
                    attr!(u64 "match_rows",              0..10_000_000),
                    attr!(f64 "hash_collision_rate",     0.0..0.3),
                    attr!(u64 "spill_count",             0..32),
                    attr!(u64 "spill_bytes",             0..4u64 * 1024 * 1024 * 1024),
                    attr!(u64 "bloom_filter_size_bytes", 0..64 * 1024 * 1024),
                    attr!(f64 "bloom_filter_fpr",        0.001..0.1),
                    // Join key columns
                    DynamicAttribute::list(
                        "join_keys",
                        DynamicList::String(
                            vec!["id", "region_id", "ts"]
                                .into_iter()
                                .take(rng().random_range(1..4))
                                .map(|s| s.to_string())
                                .collect(),
                        ),
                    ),
                    // Per-spill detail: list of structs with bytes + time
                    DynamicAttribute::list(
                        "spill_events",
                        DynamicList::Struct(
                            (0..rng().random_range(0u64..4))
                                .map(|_| {
                                    DynamicStruct(vec![
                                        DynamicAttribute::u64(
                                            "bytes",
                                            rng().random_range(1024..1024 * 1024 * 1024),
                                        ),
                                        DynamicAttribute::u64(
                                            "time_ns",
                                            rng().random_range(10_000..500_000_000),
                                        ),
                                        DynamicAttribute::u64(
                                            "rows",
                                            rng().random_range(1000..1_000_000),
                                        ),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ]),
                Physical::Sort => {
                    let num_keys: usize = rng().random_range(1..8);
                    attributes.extend([
                        attr!(pick "direction",              "asc", "desc"),
                        DynamicAttribute::u64("sort_keys", num_keys as u64),
                        attr!(u64  "comparison_count",       1000..500_000_000),
                        attr!(u64  "merge_passes",           1..16),
                        attr!(u64  "run_count",              1..512),
                        attr!(u64  "spill_count",            0..64),
                        attr!(u64  "spill_bytes",            0..4u64 * 1024 * 1024 * 1024),
                        attr!(u64  "merge_time_ns",          100_000..2_000_000_000),
                        attr!(f64  "avg_key_length_bytes",   4.0..256.0),
                        attr!(f64  "presorted_fraction",     0.0..1.0),
                        // Per sort-key specification
                        DynamicAttribute::list(
                            "key_specs",
                            DynamicList::Struct(
                                [
                                    "ts", "amount", "id", "score", "name", "region", "category",
                                    "status",
                                ]
                                .iter()
                                .take(num_keys)
                                .map(|col| {
                                    DynamicStruct(vec![
                                        DynamicAttribute::string("column", *col),
                                        DynamicAttribute::string(
                                            "direction",
                                            *rng().sample(Choose::new(&["asc", "desc"]).unwrap()),
                                        ),
                                        DynamicAttribute::string(
                                            "nulls",
                                            *rng().sample(Choose::new(&["first", "last"]).unwrap()),
                                        ),
                                    ])
                                })
                                .collect(),
                            ),
                        ),
                    ]);
                }
                Physical::Limit => attributes.extend([
                    attr!(u32 "amount",                  42),
                    attr!(u64 "rows_inspected",          42..10_000_000),
                    attr!(u64 "rows_emitted",            1..43),
                    attr!(f64 "early_termination_ratio", 0.0..1.0),
                ]),
                Physical::Output => {
                    let flush_count: u64 = rng().random_range(1..128);
                    attributes.extend([
                        attr!(pick "sink",                   "file", "memory"),
                        attr!(u64  "rows_written",           0..10_000_000),
                        attr!(u64  "bytes_written",          0..4u64 * 1024 * 1024 * 1024),
                        DynamicAttribute::u64("flush_count", flush_count),
                        attr!(u64  "flush_time_ns",          10_000..500_000_000),
                        attr!(f64  "compression_ratio",      0.1..0.9),
                        attr!(u64  "serialization_time_ns",  10_000..1_000_000_000),
                        // Per-flush durations
                        DynamicAttribute::list(
                            "per_flush_time_ns",
                            DynamicList::U64(
                                (0..flush_count)
                                    .map(|_| rng().random_range(1000..10_000_000))
                                    .collect(),
                            ),
                        ),
                    ]);
                }
            }
            let op_handle = op_obs.create(op.id);
            op_handle.statistics(operator::Statistics {
                custom_attributes: attributes.into(),
            });

            let edges = physical_plan
                .dag
                .edges_directed(*node_idx, Direction::Incoming);
            for edge in edges {
                let port = &edge.weight().target;
                let port_handle = port_obs.create(port.id);
                port_handle.statistics(port::Statistics {
                    custom_attributes: vec![
                        DynamicAttribute::u64("bytes", port.num_bytes.load(Ordering::Relaxed)),
                        DynamicAttribute::u64("rows", port.num_rows.load(Ordering::Relaxed)),
                    ]
                    .into(),
                });
            }
        }
    }

    fn shut_down(&mut self, context: &SimulatorContext) {
        let worker_obs = context.worker_observer();

        for handle in &mut self.memory_handles {
            handle.finalizing();
            handle.exit();
            sleep_long();
        }
        for handle in &mut self.channel_handles {
            handle.finalizing();
            handle.exit();
            sleep_long();
        }
        for handle in &mut self.processor_handles {
            handle.finalizing();
            handle.exit();
        }
        sleep_long();
        let worker_handle = worker_obs.create(self.id);
        worker_handle.exit(worker::Exit);
    }
}

struct Engine {
    id: Uuid,
    workers: HashMap<Uuid, Worker>,
    network: Uuid,
    network_links: HashMap<(Uuid, Uuid), Uuid>,
    network_link_handles: Vec<quent_stdlib::channel::ChannelHandle>,
}

impl Engine {
    fn new() -> Self {
        Self {
            id: Uuid::now_v7(),
            workers: Default::default(),
            network: Uuid::now_v7(),
            network_links: Default::default(),
            network_link_handles: Vec::new(),
        }
    }

    fn spawn(
        &mut self,
        context: &SimulatorContext,
        nvtx: &NvtxCapture,
        num_workers: usize,
        num_threads: usize,
    ) {
        info!("Simulating Engine:");
        info!("\thttp://localhost:8080/analyzer/engine/{}", self.id);
        let engine_obs = context.engine_observer();

        let instance_name = format!("holodeck-{:04x}", rng().random::<u32>());
        let engine_handle = engine_obs.create(self.id);
        engine_handle.init(engine::Init {
            instance_name: Some(instance_name),
            implementation: EngineImplementationAttributes {
                name: Some("Simulator".into()),
                version: Some("0.0.0-PoC".into()),
                custom_attributes: Default::default(),
            },
        });

        // Workers
        let worker_ids = std::iter::repeat_with(Uuid::now_v7)
            .take(num_workers)
            .collect::<Vec<_>>();

        for (worker_index, worker_id) in worker_ids.iter().enumerate() {
            let worker = Worker::new(
                *worker_id,
                format!("drone-{worker_index}"),
                context,
                nvtx,
                self.id,
                num_threads,
            );
            self.workers.insert(*worker_id, worker);
        }

        // Engine-wide resources
        // Create a fully connected bidirectional network of workers
        let net_obs = context.network_observer();
        self.network = Uuid::now_v7();
        net_obs.network(self.network, "network", self.id);
        let ch_obs = context.channel_observer();
        for worker_index in 0..worker_ids.len() {
            for other_worker_index in worker_index + 1..worker_ids.len() {
                let worker_id = worker_ids[worker_index];
                let other_worker_id = worker_ids[other_worker_index];

                let up_link_id = Uuid::now_v7();
                let mut up_handle = ch_obs.initializing(
                    up_link_id,
                    &format!("worker {worker_index} -> {other_worker_index}"),
                    self.network,
                    self.workers.get(&worker_id).unwrap().memory,
                    self.workers.get(&other_worker_id).unwrap().memory,
                );
                up_handle.operating(None);
                self.network_link_handles.push(up_handle);

                let down_link_id = Uuid::now_v7();
                let mut down_handle = ch_obs.initializing(
                    down_link_id,
                    &format!("worker {other_worker_index} -> {worker_index}"),
                    self.network,
                    self.workers.get(&other_worker_id).unwrap().memory,
                    self.workers.get(&worker_id).unwrap().memory,
                );
                down_handle.operating(None);
                self.network_link_handles.push(down_handle);

                self.network_links
                    .insert((worker_id, other_worker_id), up_link_id);
                self.network_links
                    .insert((other_worker_id, worker_id), down_link_id);
            }
        }
    }

    fn shut_down(&mut self, context: &SimulatorContext) {
        let engine_obs = context.engine_observer();

        // Tear down network
        for handle in &mut self.network_link_handles {
            handle.finalizing();
            handle.exit();
        }

        // Tear down workers
        for worker in self.workers.values_mut() {
            worker.shut_down(context);
        }

        let engine_handle = engine_obs.create(self.id);
        engine_handle.exit(engine::Exit);
        info!("Simulated engine shut down.")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    let args = Args::parse();

    info!("Simulating with: {args:?}");

    let mut engine = Engine::new();
    let nvtx_workload = NvtxWorkload::from_args(&args);

    let exporter = args.exporter.into_options();
    let context = match exporter.clone() {
        Some(provider) => SimulatorContext::try_new(provider)?,
        None => SimulatorContext::try_new(quent_model::Noop)?,
    };
    let nvtx_layout = NvtxLayout {
        num_domains: args.num_nvtx_domains,
        num_categories: args.num_nvtx_categories,
        num_marks: args.num_nvtx_marks,
        num_nested_ranges: args.num_nvtx_nested_ranges,
        task_every: args.nvtx_task_every,
        max_ranges: args.max_nvtx_ranges,
    };
    let nvtx = match exporter.as_ref() {
        Some(provider) => NvtxCapture::try_new(context.id(), provider, nvtx_layout)?,
        None => NvtxCapture::noop(context.id(), nvtx_layout),
    };
    nvtx.declare_schema();
    let main_thread = nvtx.name_thread("main");
    let simulation_resource = nvtx.resource(nvtx.domain_at(0), "simulated CUDA execution context");

    engine.spawn(&context, &nvtx, args.num_workers, args.num_threads);

    for (query_group_index, query_group_id) in std::iter::repeat_with(Uuid::now_v7)
        .take(args.num_query_groups)
        .enumerate()
    {
        info!("Simulating Query Group:");
        info!(
            "\thttp://localhost:8080/analyzer/engine/{}/query_group/{query_group_id}/list_queries",
            engine.id
        );

        let query_group_obs = context.query_group_observer();

        query_group_obs.declaration(
            query_group_id,
            query_group::Declaration {
                engine_id: engine.id,
                instance_name: format!("TPC-H (iteration {query_group_index})"),
            },
        );

        // "Run" the specified number of queries, sequentially for now.
        for query_index in 0..args.num_queries {
            let query_obs = context.query_observer();
            let query_id = Uuid::now_v7();
            let mut query_handle = query_obs.init(
                query_id,
                &format!("Q{query_index}"),
                Ref::new(query_group_id),
            );
            info!("Simulating Query:");
            info!(
                "\thttp://localhost:8080/analyzer/engine/{}/query/{query_id}",
                engine.id
            );
            query_handle.planning();
            let query_process = nvtx.start(
                nvtx.domain_at(0),
                &format!("Q{query_index} process range"),
                nvtx.category_id(NvtxCategory::Api),
            );
            let _query = nvtx.push(
                nvtx.domain_at(0),
                main_thread,
                "sirius::query",
                nvtx.category_id(NvtxCategory::Api),
            );
            let l_plan = {
                let _planning = nvtx.push(
                    nvtx.domain_at(0),
                    main_thread,
                    "planning",
                    nvtx.category_id(NvtxCategory::Internal),
                );
                make_logical_plan(query_id, "logical".into())
            };
            l_plan.declare(&context, None);
            query_handle.executing();
            for mark_index in 0..nvtx.layout().num_marks {
                nvtx.mark(
                    nvtx.domain_at(0),
                    &format!("Q{query_index} mark-{mark_index}"),
                    nvtx.category_at(mark_index),
                );
            }

            let workers: Vec<_> = engine.workers.values().collect();
            let nvtx_query_barrier = Barrier::new(nvtx_workload.worker_thread_count.max(1));
            let nvtx_execution = NvtxExecution {
                capture: &nvtx,
                workload: nvtx_workload,
                query_barrier: &nvtx_query_barrier,
            };
            std::thread::scope(|s| {
                for worker in workers {
                    s.spawn(|| {
                        worker.execute_logical_plan(
                            &context,
                            nvtx_execution,
                            &engine,
                            &l_plan,
                            args.num_tasks,
                        );
                    });
                }
            });
            drop(_query);
            drop(query_process);

            query_handle.exit();
        }
    }

    engine.shut_down(&context);
    drop(simulation_resource);
    nvtx.destroy_schema();

    // Each entity stream flushes only when its last observer clone is released.
    // `engine` co-owns those clones through its worker and network-link handles,
    // so it must drop together with the context to write all pending events.
    drop((engine, context, nvtx));

    info!("simulation completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvtx_budget_is_divided_across_every_query_thread() {
        let workload = NvtxWorkload {
            query_count: 2,
            worker_thread_count: 10,
        };
        let layout = NvtxLayout {
            num_domains: 3,
            max_ranges: 1_000,
            ..NvtxLayout::default()
        };
        assert_eq!(workload.detailed_ranges_per_thread(layout), 46);
    }

    #[test]
    fn default_cap_reserves_full_query_envelopes() {
        let workload = NvtxWorkload {
            query_count: 4,
            worker_thread_count: 64,
        };
        assert_eq!(
            workload.detailed_ranges_per_thread(NvtxLayout::default()),
            75
        );
    }
}
