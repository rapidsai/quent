// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    sync::{
        Barrier,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use clap::Parser;
use petgraph::{Directed, Direction, Graph, graph::NodeIndex, visit::EdgeRef};
use quent_dynamic_attributes::DynamicAttribute;
use quent_io::clap::ExporterArgs;
use quent_simulator_instrumentation::{
    self as instr, DEFAULT_MAX_NVTX_RANGES, NvtxCapture, NvtxCategory, NvtxLayout, NvtxPushGuard,
};
use rand::{RngExt, rng};
use tracing::info;
use uuid::Uuid;

type SimulatorContext = instr::Context<instr::Simulator>;

const ENGINE_ID: Uuid = Uuid::from_u128(0x01a07b4c86ab797197c124879c41910e);
const QUERY_ID_BASE: u128 = 0x01a07b4c86ab797197c128ffb10dde0d;

#[derive(Parser, Debug)]
#[command(name = "simulator")]
#[command(about = "Emits simulated query engine telemetry", long_about = None)]
struct Args {
    /// Number of simulated workloads
    #[arg(long, default_value_t = 1)]
    num_workloads: usize,

    /// Number of queries per workload
    #[arg(long, default_value_t = 1)]
    num_queries: usize,

    /// Number of tasks per operator
    #[arg(long, default_value_t = 32)]
    num_tasks: usize,

    /// Number of workers
    #[arg(long, default_value_t = 4)]
    num_workers: usize,

    /// Number of threads per worker thread pool
    #[arg(long, default_value_t = 4)]
    num_threads: usize,

    /// Number of GPUs per worker
    #[arg(long, default_value_t = 1)]
    num_gpus: usize,

    /// Generate a new engine ID instead of reusing the stable simulator ID
    #[arg(long)]
    dynamic_engine_id: bool,

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
}

impl NvtxWorkload {
    fn from_config(config: SimulationConfig) -> Self {
        Self {
            query_count: config.num_workloads.saturating_mul(config.num_queries),
            worker_thread_count: config.num_workers.saturating_mul(config.num_threads),
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

fn nvtx_range_clusters(
    nvtx: &NvtxCapture,
    slot_count: usize,
    range_budget: usize,
) -> Vec<(usize, usize)> {
    let requested_clusters = if range_budget == 0 {
        0
    } else {
        range_budget
            .div_ceil(TARGET_NVTX_RANGES_PER_CLUSTER)
            .max(2)
            .min(range_budget)
    };
    let offsets = nvtx.sampled_task_offsets(slot_count, requested_clusters);
    let cluster_count = offsets.len();
    if cluster_count == 0 {
        return Vec::new();
    }
    offsets
        .into_iter()
        .enumerate()
        .map(|(index, offset)| {
            let budget =
                range_budget / cluster_count + usize::from(index < range_budget % cluster_count);
            (offset, budget)
        })
        .collect()
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

fn sleep_fixed(micros: u64) {
    std::thread::sleep(Duration::from_micros(micros * 4));
}

/// Atomically subtract `val` from `counter`, clamping at 0 to prevent
/// unsigned underflow wrapping to u64::MAX.
fn saturating_sub(counter: &AtomicU64, val: u64) {
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        let new = current.saturating_sub(val);
        match counter.compare_exchange_weak(current, new, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

/// Simulated bandwidth limits — server-grade hardware.
const STORAGE_BANDWIDTH_MBPS: u64 = 28_000; // 28 GB/s (NVMe RAID array, 4x gen4 drives)
const PCIE_BANDWIDTH_MBPS: u64 = 63_000; // 63 GB/s (PCIe 5.0 x16)
const NETWORK_BANDWIDTH_MBPS: u64 = 50_000; // 50 GB/s (400 GbE / InfiniBand HDR)
const COMPUTE_BANDWIDTH_MBPS: u64 = 80_000; // 80 GB/s (memory-bound compute throughput)

/// Sleep to simulate a transfer at the given bandwidth (MB/s).
fn sleep_transfer(bytes: u64, bandwidth_mbps: u64) {
    let mib = (bytes / (1024 * 1024)).max(1);
    // microseconds = MiB * 1_000_000 / bandwidth_mbps
    let micros = 50 + mib * 1_000_000 / bandwidth_mbps;
    std::thread::sleep(Duration::from_micros(micros));
}

/// Storage I/O (NVMe RAID ~28 GB/s).
fn sleep_storage_io(bytes: u64) {
    sleep_transfer(bytes, STORAGE_BANDWIDTH_MBPS);
}

/// PCIe transfer: host↔GPU (~63 GB/s, PCIe 5.0 x16).
fn sleep_pcie(bytes: u64) {
    sleep_transfer(bytes, PCIE_BANDWIDTH_MBPS);
}

/// Network transfer (~50 GB/s, 400 GbE / InfiniBand).
fn sleep_network(bytes: u64) {
    sleep_transfer(bytes, NETWORK_BANDWIDTH_MBPS);
}

/// Compute-bound processing (~80 GB/s effective throughput).
fn sleep_compute(bytes: u64) {
    sleep_transfer(bytes, COMPUTE_BANDWIDTH_MBPS);
}

/// Storage I/O with occasional latency spikes (1% of the time, 4x slower).
fn sleep_storage_io_variable(bytes: u64) {
    if rng().random_ratio(1, 100) {
        sleep_transfer(bytes, STORAGE_BANDWIDTH_MBPS / 4);
    } else {
        sleep_storage_io(bytes);
    }
}

struct Operator<T: Debug> {
    handle: instr::Handle<instr::Operator>,
    parents: Vec<instr::EntityRef<instr::Operator>>,
    kind: T,
    tasks_processed: AtomicU64,
    batches_in: AtomicU64,
    bytes_in: AtomicU64,
    rows_in: AtomicU64,
    batches_out: AtomicU64,
    bytes_out: AtomicU64,
    rows_out: AtomicU64,
}

impl<T> Operator<T>
where
    T: Debug,
{
    fn name(&self) -> String {
        format!("{:?}", self.kind)
    }

    fn new(
        context: &SimulatorContext,
        kind: T,
        parents: Vec<instr::EntityRef<instr::Operator>>,
    ) -> Self {
        Self {
            handle: context.observer::<instr::Operator>().handle(),
            parents,
            kind,
            tasks_processed: AtomicU64::new(0),
            batches_in: AtomicU64::new(0),
            bytes_in: AtomicU64::new(0),
            rows_in: AtomicU64::new(0),
            batches_out: AtomicU64::new(0),
            bytes_out: AtomicU64::new(0),
            rows_out: AtomicU64::new(0),
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

struct Port {
    handle: instr::Handle<instr::Port>,
    name: &'static str,
}

struct Edge {
    source: Port,
    target: Port,
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.source.name, self.target.name)
    }
}

impl Edge {
    fn new(context: &SimulatorContext, source: &'static str, target: &'static str) -> Edge {
        let port_obs = context.observer::<instr::Port>();
        Edge {
            source: Port {
                handle: port_obs.handle(),
                name: source,
            },
            target: Port {
                handle: port_obs.handle(),
                name: target,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Logical {
    Scan,
    Project,
    Join,
    Aggregate,
    Filter,
    Udf,
    Sort,
    Limit,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Physical {
    FileSystemScan,
    GpuDecode,
    JoinPartition,
    JoinLocal,
    Aggregate,
    Filter,
    Udf,
    Sort,
    Limit,
    Output,
}

fn nvtx_pipeline_op(kind: Physical) -> &'static str {
    match kind {
        Physical::FileSystemScan => "GPU_SCAN",
        Physical::GpuDecode => "GPU_DECODE",
        Physical::JoinPartition => "PARTITION",
        Physical::JoinLocal | Physical::Aggregate => "HASH_GROUP_BY",
        Physical::Filter => "FILTER",
        Physical::Udf => "TRANSFORM",
        Physical::Sort => "ORDER_BY",
        Physical::Limit => "MERGE_SORT",
        Physical::Output => "RESULT_COLLECTOR",
    }
}

fn nvtx_execute_name(kind: Physical) -> &'static str {
    match kind {
        Physical::FileSystemScan => "read_parquet",
        Physical::GpuDecode => "decode_page_data",
        Physical::JoinPartition => "sirius_physical_partition::execute",
        Physical::JoinLocal => "sirius_physical_join::execute",
        Physical::Aggregate => "sirius_physical_grouped_aggregate::execute",
        Physical::Filter => "sirius_physical_filter::execute",
        Physical::Udf => "sirius_physical_udf::execute",
        Physical::Sort => "sirius_physical_order::execute",
        Physical::Limit => "sirius_physical_merge_sort::execute",
        Physical::Output => "sirius_physical_result_collector::execute",
    }
}

fn nvtx_category_for_operator(kind: Physical) -> NvtxCategory {
    match kind {
        Physical::FileSystemScan | Physical::Output => NvtxCategory::Io,
        Physical::JoinPartition => NvtxCategory::Memory,
        Physical::GpuDecode
        | Physical::JoinLocal
        | Physical::Aggregate
        | Physical::Filter
        | Physical::Udf
        | Physical::Sort => NvtxCategory::Compute,
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
            sleep_fixed(10);
        }
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
    sleep_fixed(10);
}

/// A work item dispatched by the scheduler to a pool thread.
struct WorkItem<'a> {
    operator_node: NodeIndex,
    operator: &'a Operator<Physical>,
    /// Input batches (empty for scan operators which produce their own).
    input_batches: Vec<Batch>,
    /// Task index for naming.
    task_index: u64,
    /// JoinLocal nodes that use selective (non-amplifying) join logic.
    selective_joins: &'a HashSet<NodeIndex>,
    /// Query-wide row budget shared by every worker's limit operator.
    result_rows: &'a AtomicU64,
}

struct PlanExecution<'a> {
    context: &'a SimulatorContext,
    nvtx: NvtxExecution<'a>,
    engine: &'a Engine,
    logical_plan: &'a Plan<Logical>,
    num_tasks: usize,
    result_rows: &'a AtomicU64,
    phase_barrier: &'a Barrier,
}

impl Physical {
    fn ends_pipeline_phase(self) -> bool {
        matches!(
            self,
            Physical::JoinPartition | Physical::Aggregate | Physical::Sort
        )
    }

    fn output_size(self, input_bytes: u64, input_rows: u64, selective_join: bool) -> (u64, u64) {
        let scale = |value: u64, percent: u64| value.saturating_mul(percent) / 100;

        match self {
            Physical::JoinLocal if selective_join => {
                let row_percent = rng().random_range(40..70);
                let byte_percent = rng().random_range(35..65);
                (
                    scale(input_bytes, byte_percent),
                    scale(input_rows, row_percent),
                )
            }
            Physical::JoinLocal => {
                let factor = rng().random_range(2..5);
                (
                    input_bytes.saturating_mul(factor),
                    input_rows.saturating_mul(factor),
                )
            }
            Physical::Aggregate => {
                let row_divisor = rng().random_range(8..24);
                let byte_divisor = rng().random_range(6..18);
                (input_bytes / byte_divisor, input_rows / row_divisor)
            }
            Physical::Filter => {
                let keep_percent = rng().random_range(35..70);
                (
                    scale(input_bytes, keep_percent),
                    scale(input_rows, keep_percent),
                )
            }
            Physical::Udf => {
                let width_percent = rng().random_range(80..95);
                (scale(input_bytes, width_percent), input_rows)
            }
            _ => (input_bytes, input_rows),
        }
    }
}

const RESULT_ROW_LIMIT: u64 = 42;

fn claim_result_rows(rows_out: &AtomicU64, requested: u64) -> u64 {
    let mut claimed = 0;
    let _ = rows_out.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        claimed = requested.min(RESULT_ROW_LIMIT.saturating_sub(current));
        (claimed > 0).then_some(current + claimed)
    });
    claimed
}

struct Plan<T>
where
    T: Debug,
{
    handle: instr::Handle<instr::Plan>,
    name: String,
    query: instr::EntityRef<instr::Query>,
    parent_plan: Option<instr::EntityRef<instr::Plan>>,
    dag: Graph<Operator<T>, Edge, Directed>,
}

impl<T: Debug> Plan<T> {
    pub fn declare(&mut self, worker: Option<instr::EntityRef<instr::Worker>>) {
        self.handle
            .declaration(
                instr::PlanParent {
                    query_id: self.query.clone(),
                    plan_id: self.parent_plan.clone(),
                },
                self.name.clone(),
                self.dag
                    .edge_references()
                    .map(|edge| instr::Edge {
                        source: edge.weight().source.handle.as_entity_ref(),
                        target: edge.weight().target.handle.as_entity_ref(),
                    })
                    .collect(),
                worker,
            )
            .unwrap();

        for node_idx in self.dag.node_indices() {
            let op = &mut self.dag[node_idx];
            op.handle
                .declaration(
                    self.handle.as_entity_ref(),
                    op.parents.clone(),
                    format!("{}:{}", node_idx.index(), op.name()),
                    op.name(),
                    Default::default(),
                )
                .unwrap();
        }

        for edge_idx in self.dag.edge_indices() {
            let (source_idx, target_idx) = self.dag.edge_endpoints(edge_idx).unwrap();
            let source_operator = self.dag[source_idx].handle.as_entity_ref();
            let target_operator = self.dag[target_idx].handle.as_entity_ref();
            let edge = &mut self.dag[edge_idx];
            edge.source
                .handle
                .declaration(source_operator, edge.source.name.to_owned())
                .unwrap();
            edge.target
                .handle
                .declaration(target_operator, edge.target.name.to_owned())
                .unwrap();
        }
    }
}

// Create the following logical plan:
// Scan -> Project \                        Scan -> Project \
//                  -> Join -> Aggregate                    -> Join -> Aggregate -> Filter -> Udf \
// Scan -> Project /                        Scan -> Project /                                     -> Join -> Filter -> Udf -> Aggregate -> Sort -> Limit -> Output
//                                                                                Scan -> Project /
// Each Scan -> Project lowers to: FileSystemScan -> GpuDecode
fn make_logical_plan(
    context: &SimulatorContext,
    query: instr::EntityRef<instr::Query>,
    name: String,
) -> Plan<Logical> {
    fn add_scan_project_branch(
        context: &SimulatorContext,
        plan: &mut Graph<Operator<Logical>, Edge, Directed>,
    ) -> NodeIndex {
        let scan = plan.add_node(Operator::new(context, Logical::Scan, vec![]));
        let project = plan.add_node(Operator::new(context, Logical::Project, vec![]));
        plan.add_edge(scan, project, Edge::new(context, "out", "in"));
        project
    }

    fn add_join(
        context: &SimulatorContext,
        plan: &mut Graph<Operator<Logical>, Edge, Directed>,
        left: NodeIndex,
        right: NodeIndex,
    ) -> NodeIndex {
        let join = plan.add_node(Operator::new(context, Logical::Join, vec![]));
        plan.add_edge(left, join, Edge::new(context, "out", "left"));
        plan.add_edge(right, join, Edge::new(context, "out", "right"));
        join
    }

    let mut dag = Graph::new();

    // Left branch: join scans A and B, then pre-aggregate
    let project_a = add_scan_project_branch(context, &mut dag);
    let project_b = add_scan_project_branch(context, &mut dag);
    let join_left = add_join(context, &mut dag, project_a, project_b);
    let agg_left = dag.add_node(Operator::new(context, Logical::Aggregate, vec![]));
    dag.add_edge(join_left, agg_left, Edge::new(context, "out", "in"));

    // Right branch: join scans C and D, then pre-aggregate
    let project_c = add_scan_project_branch(context, &mut dag);
    let project_d = add_scan_project_branch(context, &mut dag);
    let join_right = add_join(context, &mut dag, project_c, project_d);
    let agg_right = dag.add_node(Operator::new(context, Logical::Aggregate, vec![]));
    dag.add_edge(join_right, agg_right, Edge::new(context, "out", "in"));

    // Final join combining pre-aggregated sides
    let join_final = add_join(context, &mut dag, agg_left, agg_right);

    let aggregate = dag.add_node(Operator::new(context, Logical::Aggregate, vec![]));
    dag.add_edge(join_final, aggregate, Edge::new(context, "out", "in"));

    let filter = dag.add_node(Operator::new(context, Logical::Filter, vec![]));
    dag.add_edge(aggregate, filter, Edge::new(context, "out", "in"));

    let udf = dag.add_node(Operator::new(context, Logical::Udf, vec![]));
    dag.add_edge(filter, udf, Edge::new(context, "out", "in"));

    // Late-stage dimension table lookup join
    let project_e = add_scan_project_branch(context, &mut dag);
    let join_lookup = add_join(context, &mut dag, udf, project_e);

    // Post-join processing before final sort
    let post_filter = dag.add_node(Operator::new(context, Logical::Filter, vec![]));
    dag.add_edge(join_lookup, post_filter, Edge::new(context, "out", "in"));

    let post_udf = dag.add_node(Operator::new(context, Logical::Udf, vec![]));
    dag.add_edge(post_filter, post_udf, Edge::new(context, "out", "in"));

    let post_aggregate = dag.add_node(Operator::new(context, Logical::Aggregate, vec![]));
    dag.add_edge(post_udf, post_aggregate, Edge::new(context, "out", "in"));

    let sort = dag.add_node(Operator::new(context, Logical::Sort, vec![]));
    dag.add_edge(post_aggregate, sort, Edge::new(context, "out", "in"));

    let limit = dag.add_node(Operator::new(context, Logical::Limit, vec![]));
    dag.add_edge(sort, limit, Edge::new(context, "out", "in"));

    let output = dag.add_node(Operator::new(context, Logical::Output, vec![]));
    dag.add_edge(limit, output, Edge::new(context, "out", "in"));

    Plan {
        handle: context.observer::<instr::Plan>().handle(),
        name,
        query,
        parent_plan: None,
        dag,
    }
}

fn simulate_planning(context: &SimulatorContext, logical: &Plan<Logical>) -> Plan<Physical> {
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
        handle: context.observer::<instr::Plan>().handle(),
        name: "physical".into(),
        query: logical.query.clone(),
        parent_plan: Some(logical.handle.as_entity_ref()),
        dag: Graph::new(),
    };

    lower_logical(context, logical, &mut physical, output, None);

    physical
}

fn lower_logical(
    context: &SimulatorContext,
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
            // Scan+Project lowers to FileSystemScan → GpuDecode
            if let Some(scan_edge) = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .find(|edge| logical.dag[edge.source()].kind == Logical::Scan)
            {
                let scan_op = &logical.dag[scan_edge.source()];
                let scan = physical.dag.add_node(Operator::new(
                    context,
                    Physical::FileSystemScan,
                    vec![
                        current_logical_op.handle.as_entity_ref(),
                        scan_op.handle.as_entity_ref(),
                    ],
                ));
                let decode = physical.dag.add_node(Operator::new(
                    context,
                    Physical::GpuDecode,
                    vec![current_logical_op.handle.as_entity_ref()],
                ));
                physical
                    .dag
                    .add_edge(scan, decode, Edge::new(context, "out", "in"));
                if let Some((target_node, target_port)) = physical_target_idx_port {
                    physical.dag.add_edge(
                        decode,
                        target_node,
                        Edge::new(context, target_port, "in"),
                    );
                }
            } else {
                unimplemented!("this shouldn't happen in this simulator, yet");
            }
        }
        Logical::Join => {
            // split up in a partition stage and join stage
            let partition = physical.dag.add_node(Operator::new(
                context,
                Physical::JoinPartition,
                vec![current_logical_op.handle.as_entity_ref()],
            ));
            let local = physical.dag.add_node(Operator::new(
                context,
                Physical::JoinLocal,
                vec![current_logical_op.handle.as_entity_ref()],
            ));
            physical.dag.add_edge(
                partition,
                local,
                Edge::new(context, "out", "partitioned_in"),
            );

            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(local, target_node, Edge::new(context, "out", target_port));
            }

            // Recurse up both branches
            for input_edge in logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
            {
                lower_logical(
                    context,
                    logical,
                    physical,
                    input_edge.source(),
                    Some((partition, input_edge.weight().target.name)),
                );
            }
        }
        Logical::Aggregate | Logical::Filter | Logical::Udf | Logical::Sort => {
            let physical_kind = match current_logical_op.kind {
                Logical::Aggregate => Physical::Aggregate,
                Logical::Filter => Physical::Filter,
                Logical::Udf => Physical::Udf,
                Logical::Sort => Physical::Sort,
                _ => unreachable!(),
            };
            let node = physical.dag.add_node(Operator::new(
                context,
                physical_kind,
                vec![current_logical_op.handle.as_entity_ref()],
            ));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(node, target_node, Edge::new(context, "out", target_port));
            }
            let input_edge = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                context,
                logical,
                physical,
                input_edge.source(),
                Some((node, input_edge.weight().target.name)),
            );
        }
        Logical::Limit => {
            let limit = physical.dag.add_node(Operator::new(
                context,
                Physical::Limit,
                vec![current_logical_op.handle.as_entity_ref()],
            ));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(limit, target_node, Edge::new(context, "out", target_port));
            }
            let input_edge = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                context,
                logical,
                physical,
                input_edge.source(),
                Some((limit, input_edge.weight().target.name)),
            );
        }
        Logical::Output => {
            let output = physical.dag.add_node(Operator::new(
                context,
                Physical::Output,
                vec![current_logical_op.handle.as_entity_ref()],
            ));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(output, target_node, Edge::new(context, "out", target_port));
            }
            let input_edge = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .next()
                .unwrap();
            lower_logical(
                context,
                logical,
                physical,
                input_edge.source(),
                Some((output, input_edge.weight().target.name)),
            );
        }
    }
}

struct Gpu {
    id: Uuid,
    memory: Uuid,
    host_mem_to_gpu: Uuid,
    gpu_to_host_mem: Uuid,
    /// Tracks current GPU memory usage in bytes for spill decisions.
    memory_used: AtomicU64,
}

impl Gpu {
    fn new() -> Self {
        Self {
            id: Uuid::now_v7(),
            memory: Uuid::now_v7(),
            host_mem_to_gpu: Uuid::now_v7(),
            gpu_to_host_mem: Uuid::now_v7(),
            memory_used: AtomicU64::new(0),
        }
    }
}

#[derive(Clone, Debug)]
struct Batch {
    bytes: u64,
    rows: u64,
    /// Index into the worker's `gpus` vec if this batch is currently on a GPU.
    /// `None` means the batch is in host memory (or in storage if `in_storage` is true).
    gpu_index: Option<usize>,
    /// Batch has been spilled to storage; memory is not tracked on host or GPU.
    in_storage: bool,
}

enum TaskHandle {
    Queueing(instr::FsmHandle<instr::Task, instr::task_state::Queueing>),
    Allocating(instr::FsmHandle<instr::Task, instr::task_state::Allocating>),
    Loading(instr::FsmHandle<instr::Task, instr::task_state::Loading>),
    Computing(instr::FsmHandle<instr::Task, instr::task_state::Computing>),
    Spilling(instr::FsmHandle<instr::Task, instr::task_state::Spilling>),
    Sending(instr::FsmHandle<instr::Task, instr::task_state::Sending>),
    Transitioning,
}

impl TaskHandle {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Self::Transitioning)
    }

    fn allocating(
        &mut self,
        use_thread: instr::EntityRef<instr::TaskExecutorThread, instr::TaskExecutorThreadUsage>,
    ) {
        *self = match self.take() {
            Self::Queueing(handle) => Self::Allocating(handle.allocating(use_thread)),
            Self::Spilling(handle) => Self::Allocating(handle.allocating(use_thread)),
            _ => unreachable!("invalid task transition to allocating"),
        };
    }

    fn loading(
        &mut self,
        use_thread: instr::EntityRef<instr::TaskExecutorThread, instr::TaskExecutorThreadUsage>,
        use_storage_channel: Option<
            instr::EntityRef<instr::StorageChannel, instr::StorageChannelUsage>,
        >,
        use_pcie_channel: Option<instr::EntityRef<instr::PcieChannel, instr::PcieChannelUsage>>,
        use_host_memory: Option<instr::EntityRef<instr::HostMemory, instr::HostMemoryUsage>>,
        use_gpu_memory: Option<instr::EntityRef<instr::GpuMemory, instr::GpuMemoryUsage>>,
    ) {
        *self = match self.take() {
            Self::Allocating(handle) => Self::Loading(handle.loading(
                use_thread,
                use_storage_channel,
                use_pcie_channel,
                use_host_memory,
                use_gpu_memory,
            )),
            Self::Loading(handle) => Self::Loading(handle.loading(
                use_thread,
                use_storage_channel,
                use_pcie_channel,
                use_host_memory,
                use_gpu_memory,
            )),
            _ => unreachable!("invalid task transition to loading"),
        };
    }

    fn computing(
        &mut self,
        instance_name: String,
        input_bytes: u64,
        use_thread: instr::EntityRef<instr::TaskExecutorThread, instr::TaskExecutorThreadUsage>,
        use_host_memory: Option<instr::EntityRef<instr::HostMemory, instr::HostMemoryUsage>>,
        use_gpu_memory: Option<instr::EntityRef<instr::GpuMemory, instr::GpuMemoryUsage>>,
    ) {
        *self = match self.take() {
            Self::Allocating(handle) => Self::Computing(handle.computing(
                instance_name,
                input_bytes,
                use_thread,
                use_host_memory,
                use_gpu_memory,
            )),
            Self::Loading(handle) => Self::Computing(handle.computing(
                instance_name,
                input_bytes,
                use_thread,
                use_host_memory,
                use_gpu_memory,
            )),
            _ => unreachable!("invalid task transition to computing"),
        };
    }

    fn spilling(
        &mut self,
        use_thread: instr::EntityRef<instr::TaskExecutorThread, instr::TaskExecutorThreadUsage>,
        use_storage_channel: instr::EntityRef<instr::StorageChannel, instr::StorageChannelUsage>,
    ) {
        *self = match self.take() {
            Self::Computing(handle) => {
                Self::Spilling(handle.spilling(use_thread, use_storage_channel))
            }
            _ => unreachable!("invalid task transition to spilling"),
        };
    }

    fn sending(
        &mut self,
        use_thread: instr::EntityRef<instr::TaskExecutorThread, instr::TaskExecutorThreadUsage>,
        use_network_channel: instr::EntityRef<instr::NetworkChannel, instr::NetworkChannelUsage>,
    ) {
        *self = match self.take() {
            Self::Computing(handle) => {
                Self::Sending(handle.sending(use_thread, use_network_channel))
            }
            _ => unreachable!("invalid task transition to sending"),
        };
    }

    fn queueing(&mut self, instance_name: String, operator_id: instr::EntityRef<instr::Operator>) {
        *self = match self.take() {
            Self::Sending(handle) => Self::Queueing(handle.queueing(instance_name, operator_id)),
            _ => unreachable!("invalid task transition to queueing"),
        };
    }

    fn exit(mut self) {
        match self.take() {
            Self::Computing(handle) => drop(handle.exit()),
            Self::Sending(handle) => drop(handle.exit()),
            _ => unreachable!("invalid task transition to exit"),
        }
    }
}

struct Worker {
    id: Uuid,
    name: String,
    host_memory: Uuid,
    /// Tracks current host memory usage in bytes for spill decisions.
    host_memory_used: AtomicU64,
    thread_pool: Uuid,
    storage: Uuid,
    storage_to_host: Uuid,
    host_to_storage: Uuid,
    threads: Vec<Uuid>,
    nvtx_thread_ids: Vec<u32>,
    gpus: Vec<Gpu>,
}

impl Worker {
    fn new(
        id: Uuid,
        name: String,
        num_threads: usize,
        num_gpus: usize,
        nvtx: &NvtxCapture,
    ) -> Self {
        let threads = std::iter::repeat_with(Uuid::now_v7)
            .take(num_threads)
            .collect();
        let nvtx_thread_ids = (0..num_threads)
            .map(|index| nvtx.name_thread(&format!("{name} Thread {index}")))
            .collect();
        Self {
            id,
            name,
            host_memory: Uuid::now_v7(),
            host_memory_used: AtomicU64::new(0),
            thread_pool: Uuid::now_v7(),
            storage: Uuid::now_v7(),
            storage_to_host: Uuid::now_v7(),
            host_to_storage: Uuid::now_v7(),
            threads,
            nvtx_thread_ids,
            gpus: std::iter::repeat_with(Gpu::new).take(num_gpus).collect(),
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

    fn spawn(&mut self, context: &SimulatorContext, parent_engine_id: Uuid) {
        context
            .observer::<instr::Worker>()
            .handle_with_id(self.id)
            .init(
                instr::EntityRef::new(parent_engine_id, ()),
                self.name.clone(),
            )
            .unwrap();

        context
            .observer::<instr::HostMemory>()
            .handle_with_id(self.host_memory)
            .declaration("Host Memory".to_owned(), instr::EntityRef::new(self.id, ()))
            .unwrap();

        context
            .observer::<instr::Storage>()
            .handle_with_id(self.storage)
            .declaration("Storage".to_owned(), instr::EntityRef::new(self.id, ()))
            .unwrap();

        for (id, name) in [
            (self.storage_to_host, "Storage -> Host"),
            (self.host_to_storage, "Host -> Storage"),
        ] {
            context
                .observer::<instr::StorageChannel>()
                .handle_with_id(id)
                .declaration(name.to_owned(), instr::EntityRef::new(self.id, ()))
                .unwrap();
        }

        context
            .observer::<instr::TaskExecutor>()
            .handle_with_id(self.thread_pool)
            .declaration(
                "Task Executor".to_owned(),
                instr::EntityRef::new(self.id, ()),
            )
            .unwrap();
        for (index, thread_id) in self.threads.iter().enumerate() {
            context
                .observer::<instr::TaskExecutorThread>()
                .handle_with_id(*thread_id)
                .declaration(
                    format!("Thread {index}"),
                    instr::EntityRef::new(self.thread_pool, ()),
                )
                .unwrap();
        }

        for (index, gpu) in self.gpus.iter_mut().enumerate() {
            context
                .observer::<instr::Gpu>()
                .handle_with_id(gpu.id)
                .declaration(format!("GPU {index}"), instr::EntityRef::new(self.id, ()))
                .unwrap();

            context
                .observer::<instr::GpuMemory>()
                .handle_with_id(gpu.memory)
                .declaration(
                    format!("GPU {index} Memory"),
                    instr::EntityRef::new(gpu.id, ()),
                )
                .unwrap();

            for (id, name) in [
                (gpu.host_mem_to_gpu, format!("Host -> GPU {index}")),
                (gpu.gpu_to_host_mem, format!("GPU {index} -> Host")),
            ] {
                context
                    .observer::<instr::PcieChannel>()
                    .handle_with_id(id)
                    .declaration(name, instr::EntityRef::new(gpu.id, ()))
                    .unwrap();
            }
        }
    }

    /// Processes one operator for a query partition.
    ///
    /// Returns batches consumed by downstream operators in the partition.
    fn process_work_item(
        &self,
        context: &SimulatorContext,
        nvtx: &NvtxCapture,
        engine: &Engine,
        work: &WorkItem,
        thread: Uuid,
        range_budget: &mut usize,
    ) -> Vec<Batch> {
        let operator = work.operator;
        let nvtx_thread_id = self.nvtx_thread_id(thread);
        let emit_nvtx = *range_budget != 0;
        let category = nvtx.category_id(nvtx_category_for_operator(operator.kind));
        let _pipeline = emit_nvtx.then(|| {
            nvtx_push_budgeted(
                nvtx,
                range_budget,
                nvtx.domain_at(0),
                nvtx_thread_id,
                &format!(
                    "Pipeline 0: {} (id={})",
                    nvtx_pipeline_op(operator.kind),
                    operator.handle.id()
                ),
                category,
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
        if emit_nvtx {
            if operator.kind == Physical::FileSystemScan {
                nvtx_libcudf_scan(nvtx, nvtx_thread_id, range_budget);
            } else if matches!(
                operator.kind,
                Physical::GpuDecode
                    | Physical::JoinPartition
                    | Physical::JoinLocal
                    | Physical::Aggregate
                    | Physical::Filter
                    | Physical::Udf
                    | Physical::Sort
            ) {
                nvtx_cccl_kernel(nvtx, nvtx_thread_id, work.task_index as usize, range_budget);
            }
        }
        let mut task = TaskHandle::Queueing(context.observer::<instr::Task>().handle().queueing(
            format!("task-{}", work.task_index),
            operator.handle.as_entity_ref(),
        ));
        sleep_fixed(50);

        let mut input_batches = if operator.kind == Physical::FileSystemScan {
            let compressed_bytes = rng().random_range(8..32) * 1024 * 1024;
            let rows = rng().random_range(32_768..131_072);
            self.host_memory_used
                .fetch_add(compressed_bytes, Ordering::Relaxed);
            vec![Batch {
                bytes: compressed_bytes,
                rows,
                gpu_index: None,
                in_storage: false,
            }]
        } else {
            work.input_batches.clone()
        };

        let input_bytes: u64 = input_batches.iter().map(|batch| batch.bytes).sum();
        let input_rows: u64 = input_batches.iter().map(|batch| batch.rows).sum();
        operator
            .batches_in
            .fetch_add(input_batches.len() as u64, Ordering::Relaxed);
        operator.bytes_in.fetch_add(input_bytes, Ordering::Relaxed);
        operator.rows_in.fetch_add(input_rows, Ordering::Relaxed);

        let uses_gpu = matches!(
            operator.kind,
            Physical::GpuDecode
                | Physical::JoinPartition
                | Physical::JoinLocal
                | Physical::Aggregate
                | Physical::Filter
                | Physical::Udf
                | Physical::Sort
        );
        let gpu_index =
            (uses_gpu && !self.gpus.is_empty()).then(|| work.task_index as usize % self.gpus.len());
        let gpu = gpu_index.and_then(|index| self.gpus.get(index));
        let thread_usage = || instr::EntityRef::new(thread, instr::TaskExecutorThreadUsage);
        let host_memory_usage =
            |bytes| instr::EntityRef::new(self.host_memory, instr::HostMemoryUsage { bytes });
        let gpu_memory_usage =
            |id, bytes| instr::EntityRef::new(id, instr::GpuMemoryUsage { bytes });

        task.allocating(thread_usage());
        sleep_fixed(25);
        let reads_storage = operator.kind == Physical::FileSystemScan
            || input_batches.iter().any(|batch| batch.in_storage);
        if reads_storage {
            task.loading(
                thread_usage(),
                Some(instr::EntityRef::new(
                    self.storage_to_host,
                    instr::StorageChannelUsage { bytes: input_bytes },
                )),
                None,
                Some(host_memory_usage(input_bytes)),
                None,
            );
            sleep_storage_io_variable(input_bytes);
            for batch in &mut input_batches {
                batch.in_storage = false;
                batch.gpu_index = None;
            }
        }

        if let Some(target_gpu_index) = gpu_index {
            for source_gpu_index in 0..self.gpus.len() {
                if source_gpu_index == target_gpu_index {
                    continue;
                }
                let transfer_bytes: u64 = input_batches
                    .iter()
                    .filter(|batch| batch.gpu_index == Some(source_gpu_index))
                    .map(|batch| batch.bytes)
                    .sum();
                if transfer_bytes == 0 {
                    continue;
                }
                let source_gpu = &self.gpus[source_gpu_index];
                task.loading(
                    thread_usage(),
                    None,
                    Some(instr::EntityRef::new(
                        source_gpu.gpu_to_host_mem,
                        instr::PcieChannelUsage {
                            bytes: transfer_bytes,
                        },
                    )),
                    Some(host_memory_usage(transfer_bytes)),
                    None,
                );
                sleep_pcie(transfer_bytes);
                saturating_sub(&source_gpu.memory_used, transfer_bytes);
                self.host_memory_used
                    .fetch_add(transfer_bytes, Ordering::Relaxed);
                for batch in &mut input_batches {
                    if batch.gpu_index == Some(source_gpu_index) {
                        batch.gpu_index = None;
                    }
                }
            }

            let host_to_gpu_bytes: u64 = input_batches
                .iter()
                .filter(|batch| batch.gpu_index.is_none())
                .map(|batch| batch.bytes)
                .sum();
            if host_to_gpu_bytes > 0 {
                let target_gpu = &self.gpus[target_gpu_index];
                task.loading(
                    thread_usage(),
                    None,
                    Some(instr::EntityRef::new(
                        target_gpu.host_mem_to_gpu,
                        instr::PcieChannelUsage {
                            bytes: host_to_gpu_bytes,
                        },
                    )),
                    None,
                    Some(gpu_memory_usage(target_gpu.memory, host_to_gpu_bytes)),
                );
                sleep_pcie(host_to_gpu_bytes);
                saturating_sub(&self.host_memory_used, host_to_gpu_bytes);
                target_gpu
                    .memory_used
                    .fetch_add(host_to_gpu_bytes, Ordering::Relaxed);
                for batch in &mut input_batches {
                    if batch.gpu_index.is_none() {
                        batch.gpu_index = Some(target_gpu_index);
                    }
                }
            }
        } else {
            for source_gpu_index in 0..self.gpus.len() {
                let transfer_bytes: u64 = input_batches
                    .iter()
                    .filter(|batch| batch.gpu_index == Some(source_gpu_index))
                    .map(|batch| batch.bytes)
                    .sum();
                if transfer_bytes == 0 {
                    continue;
                }
                let source_gpu = &self.gpus[source_gpu_index];
                task.loading(
                    thread_usage(),
                    None,
                    Some(instr::EntityRef::new(
                        source_gpu.gpu_to_host_mem,
                        instr::PcieChannelUsage {
                            bytes: transfer_bytes,
                        },
                    )),
                    Some(host_memory_usage(transfer_bytes)),
                    None,
                );
                sleep_pcie(transfer_bytes);
                saturating_sub(&source_gpu.memory_used, transfer_bytes);
                self.host_memory_used
                    .fetch_add(transfer_bytes, Ordering::Relaxed);
                for batch in &mut input_batches {
                    if batch.gpu_index == Some(source_gpu_index) {
                        batch.gpu_index = None;
                    }
                }
            }
        }
        let working_bytes = (input_bytes / 2).clamp(1024 * 1024, 2 * 1024 * 1024 * 1024);
        if let Some(device) = gpu {
            device
                .memory_used
                .fetch_add(working_bytes, Ordering::Relaxed);
        } else {
            self.host_memory_used
                .fetch_add(working_bytes, Ordering::Relaxed);
        }
        task.computing(
            operator.name(),
            input_bytes,
            thread_usage(),
            gpu.is_none().then(|| host_memory_usage(working_bytes)),
            gpu.map(|device| gpu_memory_usage(device.memory, working_bytes)),
        );
        let compute_multiplier = match operator.kind {
            Physical::JoinLocal => 3,
            Physical::Aggregate | Physical::Sort => 2,
            _ => 1,
        };
        sleep_compute(input_bytes.saturating_mul(compute_multiplier));
        if let Some(device) = gpu {
            saturating_sub(&device.memory_used, working_bytes);
        } else {
            saturating_sub(&self.host_memory_used, working_bytes);
        }
        for batch in &input_batches {
            if let Some(index) = batch.gpu_index {
                saturating_sub(&self.gpus[index].memory_used, batch.bytes);
            } else if !batch.in_storage {
                saturating_sub(&self.host_memory_used, batch.bytes);
            }
        }

        if operator.kind == Physical::Output {
            if input_bytes > 0 {
                task.spilling(
                    thread_usage(),
                    instr::EntityRef::new(
                        self.host_to_storage,
                        instr::StorageChannelUsage { bytes: input_bytes },
                    ),
                );
                sleep_storage_io(input_bytes);
                task.allocating(thread_usage());
                task.computing(
                    "finalize output".to_owned(),
                    0,
                    thread_usage(),
                    Some(host_memory_usage(0)),
                    None,
                );
            }
            task.exit();
            operator.tasks_processed.fetch_add(1, Ordering::Relaxed);
            return Vec::new();
        }

        let (mut output_bytes, output_rows) = match operator.kind {
            Physical::GpuDecode => (
                input_bytes.saturating_mul(rng().random_range(2..4)),
                input_rows,
            ),
            Physical::Limit => {
                let rows = claim_result_rows(work.result_rows, input_rows);
                let bytes = input_bytes
                    .saturating_mul(rows)
                    .checked_div(input_rows)
                    .unwrap_or(0);
                (bytes, rows)
            }
            _ => operator.kind.output_size(
                input_bytes,
                input_rows,
                work.selective_joins.contains(&work.operator_node),
            ),
        };

        if output_rows == 0 {
            task.exit();
            operator.tasks_processed.fetch_add(1, Ordering::Relaxed);
            return Vec::new();
        }
        output_bytes = output_bytes.max(output_rows.saturating_mul(8));
        operator
            .bytes_out
            .fetch_add(output_bytes, Ordering::Relaxed);
        operator.rows_out.fetch_add(output_rows, Ordering::Relaxed);

        if operator.kind == Physical::JoinPartition {
            operator.batches_out.fetch_add(1, Ordering::Relaxed);
            if let Some(device) = gpu {
                device
                    .memory_used
                    .fetch_add(output_bytes, Ordering::Relaxed);
            } else {
                self.host_memory_used
                    .fetch_add(output_bytes, Ordering::Relaxed);
            }
            let batch = Batch {
                bytes: output_bytes,
                rows: output_rows,
                gpu_index,
                in_storage: false,
            };
            if engine.workers.len() > 1
                && let Some(other) = engine.workers.keys().find(|id| **id != self.id)
            {
                let link = &engine.network_links[&(self.id, *other)];
                let network_bytes = output_bytes
                    .saturating_mul(engine.workers.len().saturating_sub(1) as u64)
                    / engine.workers.len() as u64;
                task.sending(
                    thread_usage(),
                    instr::EntityRef::new(
                        *link,
                        instr::NetworkChannelUsage {
                            bytes: network_bytes,
                        },
                    ),
                );
                sleep_network(network_bytes);
                task.queueing(
                    "network completion".to_owned(),
                    operator.handle.as_entity_ref(),
                );
                task.allocating(thread_usage());
                task.computing(
                    "finalize shuffle".to_owned(),
                    0,
                    thread_usage(),
                    gpu.is_none().then(|| host_memory_usage(0)),
                    gpu.map(|device| gpu_memory_usage(device.memory, 0)),
                );
            }
            task.exit();
            operator.tasks_processed.fetch_add(1, Ordering::Relaxed);
            return vec![batch];
        }

        const MAX_CHUNK_BYTES: u64 = 64 * 1024 * 1024;
        let chunk_count = output_bytes.div_ceil(MAX_CHUNK_BYTES).max(1);
        operator
            .batches_out
            .fetch_add(chunk_count, Ordering::Relaxed);
        let batches: Vec<Batch> = (0..chunk_count)
            .map(|index| {
                let batch = Batch {
                    bytes: output_bytes / chunk_count
                        + u64::from(index < output_bytes % chunk_count),
                    rows: output_rows / chunk_count + u64::from(index < output_rows % chunk_count),
                    gpu_index,
                    in_storage: false,
                };
                if let Some(device) = gpu {
                    device.memory_used.fetch_add(batch.bytes, Ordering::Relaxed);
                } else {
                    self.host_memory_used
                        .fetch_add(batch.bytes, Ordering::Relaxed);
                }
                batch
            })
            .collect();

        task.exit();
        operator.tasks_processed.fetch_add(1, Ordering::Relaxed);
        batches
    }

    fn execute_logical_plan(&self, execution: PlanExecution<'_>) {
        let PlanExecution {
            context,
            nvtx,
            engine,
            logical_plan,
            num_tasks,
            result_rows,
            phase_barrier,
        } = execution;
        let mut physical_plan = simulate_planning(context, logical_plan);
        physical_plan.declare(Some(instr::EntityRef::new(self.id, ())));
        let nodes = petgraph::algo::toposort(&physical_plan.dag, None).unwrap();
        let first_join = nodes
            .iter()
            .copied()
            .find(|node| physical_plan.dag[*node].kind == Physical::JoinLocal);
        let selective_joins: HashSet<NodeIndex> = nodes
            .iter()
            .copied()
            .filter(|node| {
                physical_plan.dag[*node].kind == Physical::JoinLocal && Some(*node) != first_join
            })
            .collect();
        let mut phases = vec![Vec::new()];
        for &node in &nodes {
            phases.last_mut().unwrap().push(node);
            if physical_plan.dag[node].kind.ends_pipeline_phase() {
                phases.push(Vec::new());
            }
        }
        phases.retain(|phase| !phase.is_empty());

        std::thread::scope(|scope| {
            for (thread_index, &thread) in self.threads.iter().enumerate() {
                let physical_plan = &physical_plan;
                let phases = &phases;
                let selective_joins = &selective_joins;
                scope.spawn(move || {
                    let nvtx_thread_id = self.nvtx_thread_id(thread);
                    let _thread_envelopes: Vec<_> = (0..nvtx.capture.layout().num_domains)
                        .map(|domain_index| {
                            nvtx.capture.push(
                                nvtx.capture.domain_at(domain_index),
                                nvtx_thread_id,
                                "worker query execution",
                                nvtx.capture.category_id(NvtxCategory::Api),
                            )
                        })
                        .collect();
                    let mut partitions: Vec<_> = (thread_index..num_tasks)
                        .step_by(self.threads.len())
                        .map(|task_index| (task_index, HashMap::new()))
                        .collect();
                    let range_clusters = nvtx_range_clusters(
                        nvtx.capture,
                        phases.len().saturating_mul(partitions.len()),
                        nvtx.workload
                            .detailed_ranges_per_thread(nvtx.capture.layout()),
                    );
                    let mut next_cluster = 0;
                    let mut slot_offset = 0;
                    let mut range_budget = 0usize;

                    for phase in phases {
                        for (task_index, outputs) in &mut partitions {
                            if let Some(&(offset, budget)) = range_clusters.get(next_cluster)
                                && offset == slot_offset
                            {
                                range_budget = range_budget.saturating_add(budget);
                                next_cluster += 1;
                            }
                            for &node in phase {
                                let operator = &physical_plan.dag[node];
                                let input_batches = physical_plan
                                    .dag
                                    .edges_directed(node, Direction::Incoming)
                                    .flat_map(|edge| {
                                        outputs.get(&edge.source()).into_iter().flatten().cloned()
                                    })
                                    .collect();
                                let work = WorkItem {
                                    operator_node: node,
                                    operator,
                                    input_batches,
                                    task_index: *task_index as u64,
                                    selective_joins,
                                    result_rows,
                                };
                                let node_outputs = self.process_work_item(
                                    context,
                                    nvtx.capture,
                                    engine,
                                    &work,
                                    thread,
                                    &mut range_budget,
                                );
                                outputs.insert(node, node_outputs);
                            }
                            slot_offset += 1;
                        }
                        phase_barrier.wait();
                    }
                });
            }
        });

        for &node in &nodes {
            let operator = &mut physical_plan.dag[node];
            let input_batches = operator.batches_in.load(Ordering::Relaxed);
            let input_bytes = operator.bytes_in.load(Ordering::Relaxed);
            let input_rows = operator.rows_in.load(Ordering::Relaxed);
            let output_batches = operator.batches_out.load(Ordering::Relaxed);
            let output_bytes = operator.bytes_out.load(Ordering::Relaxed);
            let output_rows = operator.rows_out.load(Ordering::Relaxed);
            let mut attributes = vec![
                DynamicAttribute::u64(
                    "tasks_processed",
                    operator.tasks_processed.load(Ordering::Relaxed),
                ),
                DynamicAttribute::u64("input_batches", input_batches),
                DynamicAttribute::u64("input_bytes", input_bytes),
                DynamicAttribute::u64("input_rows", input_rows),
                DynamicAttribute::u64("output_batches", output_batches),
                DynamicAttribute::u64("output_bytes", output_bytes),
                DynamicAttribute::u64("output_rows", output_rows),
            ];
            match operator.kind {
                Physical::FileSystemScan => {
                    attributes.push(DynamicAttribute::u64("bytes_read", input_bytes))
                }
                Physical::GpuDecode => attributes.extend([
                    DynamicAttribute::u64("compressed_bytes", input_bytes),
                    DynamicAttribute::u64("decompressed_bytes", output_bytes),
                ]),
                Physical::JoinPartition => attributes.push(DynamicAttribute::u64(
                    "network_bytes_sent",
                    output_bytes.saturating_mul(engine.workers.len().saturating_sub(1) as u64)
                        / engine.workers.len().max(1) as u64,
                )),
                Physical::JoinLocal => attributes.extend([
                    DynamicAttribute::u64("build_rows", input_rows / 2),
                    DynamicAttribute::u64("probe_rows", input_rows - input_rows / 2),
                    DynamicAttribute::u64("match_rows", output_rows),
                ]),
                Physical::Aggregate => attributes.push(DynamicAttribute::f64(
                    "reduction_factor",
                    input_rows as f64 / output_rows.max(1) as f64,
                )),
                Physical::Filter => attributes.push(DynamicAttribute::f64(
                    "selectivity",
                    output_rows as f64 / input_rows.max(1) as f64,
                )),
                Physical::Limit => {
                    attributes.push(DynamicAttribute::u64("amount", RESULT_ROW_LIMIT))
                }
                Physical::Output => {
                    attributes.push(DynamicAttribute::u64("rows_written", input_rows))
                }
                Physical::Udf | Physical::Sort => {}
            }
            operator.handle.statistics(attributes.into()).unwrap();
        }

        for edge_idx in physical_plan.dag.edge_indices() {
            let (source_idx, _) = physical_plan.dag.edge_endpoints(edge_idx).unwrap();
            let source = &physical_plan.dag[source_idx];
            let bytes = source.bytes_out.load(Ordering::Relaxed);
            let rows = source.rows_out.load(Ordering::Relaxed);
            let attributes = || {
                vec![
                    DynamicAttribute::u64("bytes", bytes),
                    DynamicAttribute::u64("rows", rows),
                ]
                .into()
            };
            let edge = &mut physical_plan.dag[edge_idx];
            edge.source.handle.statistics(attributes()).unwrap();
            edge.target.handle.statistics(attributes()).unwrap();
        }
    }

    fn shut_down(&mut self, context: &SimulatorContext) {
        context
            .observer::<instr::Worker>()
            .handle_with_id(self.id)
            .exit()
            .unwrap();
    }
}

struct Engine {
    handle: instr::Handle<instr::Engine>,
    workers: HashMap<Uuid, Worker>,
    network: instr::Handle<instr::Network>,
    network_links: HashMap<(Uuid, Uuid), Uuid>,
}

impl Engine {
    fn new(context: &SimulatorContext, id: Uuid) -> Self {
        Self {
            handle: context.observer::<instr::Engine>().handle_with_id(id),
            workers: Default::default(),
            network: context.observer::<instr::Network>().handle(),
            network_links: Default::default(),
        }
    }

    fn spawn(&mut self, context: &SimulatorContext, nvtx: &NvtxCapture, config: SimulationConfig) {
        info!("Simulating Engine {}", self.handle.id());
        self.handle
            .init(
                instr::EngineImplementationAttributes {
                    name: Some("Simulator".into()),
                    version: Some("vibe".into()),
                    custom_attributes: Default::default(),
                },
                Some(format!("holodeck-{:04x}", rng().random::<u32>())),
                instr::EngineConfiguration {
                    workers: config.num_workers as u64,
                    threads_per_worker: config.num_threads as u64,
                    gpus_per_worker: config.num_gpus as u64,
                    num_workloads: config.num_workloads as u64,
                    num_queries: config.num_workloads as u64 * config.num_queries as u64,
                },
            )
            .unwrap();

        // Workers
        let worker_ids = std::iter::repeat_with(Uuid::now_v7)
            .take(config.num_workers)
            .collect::<Vec<_>>();

        for (worker_index, worker_id) in worker_ids.iter().enumerate() {
            let mut worker = Worker::new(
                *worker_id,
                format!("worker-{worker_index}"),
                config.num_threads,
                config.num_gpus,
                nvtx,
            );
            worker.spawn(context, self.handle.id());
            self.workers.insert(*worker_id, worker);
        }

        // Engine-wide resources
        // Create a fully connected bidirectional network of workers
        self.network
            .declaration("Network".to_owned(), self.handle.as_entity_ref())
            .unwrap();
        let channel_obs = context.observer::<instr::NetworkChannel>();
        for worker_index in 0..worker_ids.len() {
            for other_worker_index in worker_index + 1..worker_ids.len() {
                let worker_id = worker_ids[worker_index];
                let other_worker_id = worker_ids[other_worker_index];
                let up_link = Uuid::now_v7();
                channel_obs
                    .handle_with_id(up_link)
                    .declaration(
                        format!("worker {worker_index} -> {other_worker_index}"),
                        self.network.as_entity_ref(),
                    )
                    .unwrap();
                self.network_links
                    .insert((worker_id, other_worker_id), up_link);

                let down_link = Uuid::now_v7();
                channel_obs
                    .handle_with_id(down_link)
                    .declaration(
                        format!("worker {other_worker_index} -> {worker_index}"),
                        self.network.as_entity_ref(),
                    )
                    .unwrap();
                self.network_links
                    .insert((other_worker_id, worker_id), down_link);
            }
        }
    }

    fn shut_down(&mut self, context: &SimulatorContext) {
        // Tear down workers
        for worker in self.workers.values_mut() {
            worker.shut_down(context);
        }

        self.handle.exit().unwrap();
        info!("Simulated engine shut down.")
    }
}

/// Controls the amount of telemetry emitted by a simulator run.
#[derive(Clone, Copy, Debug)]
pub struct SimulationConfig {
    /// Number of simulated workloads.
    pub num_workloads: usize,
    /// Number of queries in each workload.
    pub num_queries: usize,
    /// Number of tasks per operator.
    pub num_tasks: usize,
    /// Number of workers.
    pub num_workers: usize,
    /// Number of threads in each worker thread pool.
    pub num_threads: usize,
    /// Number of GPUs attached to each worker.
    pub num_gpus: usize,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            num_workloads: 1,
            num_queries: 1,
            num_tasks: 32,
            num_workers: 4,
            num_threads: 4,
            num_gpus: 1,
        }
    }
}

/// Emits a simulator run through `context`.
pub fn simulate(context: SimulatorContext, config: SimulationConfig) {
    let nvtx = NvtxCapture::noop(context.id(), NvtxLayout::default());
    simulate_with_nvtx(context, config, ENGINE_ID, nvtx);
}

fn simulate_with_nvtx(
    context: SimulatorContext,
    config: SimulationConfig,
    engine_id: Uuid,
    nvtx: NvtxCapture,
) {
    let nvtx_workload = NvtxWorkload::from_config(config);
    nvtx.declare_schema();
    let main_thread = nvtx.name_thread("main");
    let simulation_resource = nvtx.resource(nvtx.domain_at(0), "simulated CUDA execution context");

    let mut engine = Engine::new(&context, engine_id);
    engine.spawn(&context, &nvtx, config);

    for workload_index in 0..config.num_workloads {
        let workload_name = format!("Simulated workload (run {workload_index})");
        // "Run" the specified number of queries, sequentially for now.
        for query_index in 0..config.num_queries {
            let total = config.num_workloads * config.num_queries;
            let done = workload_index * config.num_queries + query_index;
            let query_id = Uuid::from_u128(QUERY_ID_BASE + done as u128);
            info!("{}% ({}/{})", done * 100 / total, done, total);
            const QUERY_NUMBERS: &[u32] = &[42, 1337, 7, 404, 256, 99, 13, 1024, 69, 314];
            let n = QUERY_NUMBERS[query_index % QUERY_NUMBERS.len()];
            let query_name = format!("Q{n}");
            let query_process = nvtx.start(
                nvtx.domain_at(0),
                &format!("Q{query_index} process range"),
                nvtx.category_id(NvtxCategory::Api),
            );
            let query_range = nvtx.push(
                nvtx.domain_at(0),
                main_thread,
                "sirius::query",
                nvtx.category_id(NvtxCategory::Api),
            );
            let query = context
                .observer::<instr::Query>()
                .handle_with_id(query_id)
                .init(
                    query_name,
                    engine.handle.as_entity_ref(),
                    workload_name.clone(),
                    query_index as u64,
                )
                .planning();
            let mut l_plan = {
                let _planning = nvtx.push(
                    nvtx.domain_at(0),
                    main_thread,
                    "planning",
                    nvtx.category_id(NvtxCategory::Internal),
                );
                make_logical_plan(&context, query.as_entity_ref(), "logical".into())
            };
            l_plan.declare(None);
            let query = query.executing();
            for mark_index in 0..nvtx.layout().num_marks {
                nvtx.mark(
                    nvtx.domain_at(0),
                    &format!("Q{query_index} mark-{mark_index}"),
                    nvtx.category_at(mark_index),
                );
            }

            let workers: Vec<_> = engine.workers.values().collect();
            let result_rows = AtomicU64::new(0);
            let phase_barrier =
                Barrier::new(workers.iter().map(|worker| worker.threads.len()).sum());
            std::thread::scope(|s| {
                let context = &context;
                let engine = &engine;
                let l_plan = &l_plan;
                let result_rows = &result_rows;
                let phase_barrier = &phase_barrier;
                let nvtx = &nvtx;
                for worker in workers {
                    s.spawn(move || {
                        worker.execute_logical_plan(PlanExecution {
                            context,
                            nvtx: NvtxExecution {
                                capture: nvtx,
                                workload: nvtx_workload,
                            },
                            engine,
                            logical_plan: l_plan,
                            num_tasks: config.num_tasks,
                            result_rows,
                            phase_barrier,
                        });
                    });
                }
            });
            drop(query_range);
            drop(query_process);

            drop(query.done());
        }
    }

    engine.shut_down(&context);
    drop(simulation_resource);
    nvtx.destroy_schema();

    drop((engine, context, nvtx));
    info!("simulation completed");
}

/// Runs the simulator command-line interface.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    let args = Args::parse();

    if args.num_workers == 0 || args.num_threads == 0 || args.num_tasks == 0 {
        return Err("num-workers, num-threads, and num-tasks must be greater than zero".into());
    }

    info!("Simulating with: {args:?}");

    let config = SimulationConfig {
        num_workloads: args.num_workloads,
        num_queries: args.num_queries,
        num_tasks: args.num_tasks,
        num_workers: args.num_workers,
        num_threads: args.num_threads,
        num_gpus: args.num_gpus,
    };
    let nvtx_layout = NvtxLayout {
        num_domains: args.num_nvtx_domains,
        num_categories: args.num_nvtx_categories,
        num_marks: args.num_nvtx_marks,
        num_nested_ranges: args.num_nvtx_nested_ranges,
        task_every: args.nvtx_task_every,
        max_ranges: args.max_nvtx_ranges,
    };
    let exporter = args.exporter.into_options();
    let context = match exporter.clone() {
        Some(provider) => SimulatorContext::try_new(provider)?,
        None => SimulatorContext::try_new(instr::Noop)?,
    };
    let engine_id = if args.dynamic_engine_id {
        Uuid::now_v7()
    } else {
        ENGINE_ID
    };
    let nvtx = match exporter.as_ref() {
        Some(provider) => NvtxCapture::try_new(context.id(), provider, nvtx_layout)?,
        None => NvtxCapture::noop(context.id(), nvtx_layout),
    };
    simulate_with_nvtx(context, config, engine_id, nvtx);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dynamic_engine_id_flag() {
        assert!(
            !Args::try_parse_from(["simulator"])
                .unwrap()
                .dynamic_engine_id
        );
        assert!(
            Args::try_parse_from(["simulator", "--dynamic-engine-id"])
                .unwrap()
                .dynamic_engine_id
        );
    }

    #[test]
    fn reducing_operators_do_not_increase_data_volume() {
        const BYTES: u64 = 1_000_000;
        const ROWS: u64 = 100_000;

        for _ in 0..100 {
            for kind in [
                Physical::JoinLocal,
                Physical::Aggregate,
                Physical::Filter,
                Physical::Udf,
            ] {
                let (bytes, rows) = kind.output_size(BYTES, ROWS, true);
                assert!(bytes < BYTES, "{kind:?} increased or preserved bytes");
                assert!(rows <= ROWS, "{kind:?} increased rows");
            }
        }
    }

    #[test]
    fn amplifying_join_and_sort_have_realistic_cardinality() {
        let (join_bytes, join_rows) = Physical::JoinLocal.output_size(1_000_000, 100_000, false);
        assert!(join_bytes > 1_000_000);
        assert!(join_rows > 100_000);

        assert_eq!(
            Physical::Sort.output_size(1_000_000, 100_000, false),
            (1_000_000, 100_000)
        );
    }

    #[test]
    fn blocking_operators_end_pipeline_phases() {
        for kind in [Physical::JoinPartition, Physical::Aggregate, Physical::Sort] {
            assert!(kind.ends_pipeline_phase(), "{kind:?} must be a barrier");
        }

        for kind in [
            Physical::FileSystemScan,
            Physical::GpuDecode,
            Physical::JoinLocal,
            Physical::Filter,
            Physical::Udf,
            Physical::Limit,
            Physical::Output,
        ] {
            assert!(
                !kind.ends_pipeline_phase(),
                "{kind:?} must remain pipelined"
            );
        }
    }

    #[test]
    fn concurrent_limit_claims_cannot_exceed_result_limit() {
        let rows_out = AtomicU64::new(0);
        let claimed = AtomicU64::new(0);

        std::thread::scope(|scope| {
            for _ in 0..16 {
                scope.spawn(|| {
                    claimed.fetch_add(claim_result_rows(&rows_out, 10), Ordering::Relaxed);
                });
            }
        });

        assert_eq!(claimed.load(Ordering::Relaxed), RESULT_ROW_LIMIT);
        assert_eq!(rows_out.load(Ordering::Relaxed), RESULT_ROW_LIMIT);
    }

    #[test]
    fn downstream_pipeline_reduces_data_before_output() {
        const INPUT_BYTES: u64 = 10 * 1024 * 1024 * 1024;
        const INPUT_ROWS: u64 = 100_000_000;

        for _ in 0..100 {
            let (joined_bytes, joined_rows) =
                Physical::JoinLocal.output_size(INPUT_BYTES, INPUT_ROWS, true);
            let (filtered_bytes, filtered_rows) =
                Physical::Filter.output_size(joined_bytes, joined_rows, false);
            let (udf_bytes, udf_rows) =
                Physical::Udf.output_size(filtered_bytes, filtered_rows, false);
            let (aggregate_bytes, aggregate_rows) =
                Physical::Aggregate.output_size(udf_bytes, udf_rows, false);
            let sorted = Physical::Sort.output_size(aggregate_bytes, aggregate_rows, false);

            assert!(joined_bytes < INPUT_BYTES);
            assert!(filtered_bytes < joined_bytes);
            assert!(udf_bytes < filtered_bytes);
            assert!(aggregate_bytes < udf_bytes);
            assert!(aggregate_bytes < INPUT_BYTES / 10);
            assert_eq!(sorted, (aggregate_bytes, aggregate_rows));
        }
    }

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
    fn default_nvtx_cap_reserves_query_envelopes() {
        let workload = NvtxWorkload {
            query_count: 4,
            worker_thread_count: 64,
        };
        assert_eq!(
            workload.detailed_ranges_per_thread(NvtxLayout::default()),
            75
        );
    }

    #[test]
    fn nvtx_range_clusters_span_the_execution() {
        let nvtx = NvtxCapture::noop(Uuid::nil(), NvtxLayout::default());
        let clusters = nvtx_range_clusters(&nvtx, 100, 96);

        assert_eq!(clusters, vec![(0, 32), (49, 32), (99, 32)]);
        assert_eq!(clusters.iter().map(|(_, budget)| budget).sum::<usize>(), 96);
    }
}
