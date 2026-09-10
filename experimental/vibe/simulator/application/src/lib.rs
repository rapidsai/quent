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
use quent_model::{Ref, usage};
use quent_query_engine_model::{
    engine::{self, EngineImplementationAttributes},
    operator, plan, port, query_group, worker,
};
use quent_simulator_instrumentation::{
    DEFAULT_MAX_NVTX_RANGES, NvtxCapture, NvtxCategory, NvtxLayout, NvtxPushGuard, SimulatorContext,
};
use rand::{RngExt, rng};
use tracing::info;
use uuid::Uuid;

const ENGINE_ID: Uuid = Uuid::from_u128(0x01a07b4c86ab797197c124879c41910e);
const QUERY_ID_BASE: u128 = 0x01a07b4c86ab797197c128ffb10dde0d;

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
    #[arg(long, default_value_t = 4)]
    num_workers: usize,

    /// Number of threads per worker thread pool
    #[arg(long, default_value_t = 4)]
    num_threads: usize,

    /// Number of GPUs per worker
    #[arg(long, default_value_t = 1)]
    num_gpus: usize,

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
            query_count: config.num_query_groups.saturating_mul(config.num_queries),
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
    id: Uuid,
    parents: Vec<Uuid>,
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

    fn new(kind: T, parents: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::now_v7(),
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

#[derive(Debug)]
struct Port {
    id: Uuid,
    name: &'static str,
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
            },
            target: Port {
                id: Uuid::now_v7(),
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
    id: Uuid,
    name: String,
    query_id: Uuid,
    parent_plan_id: Option<Uuid>,
    dag: Graph<Operator<T>, Edge, Directed>,
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
                parent_operator_ids: op.parents.iter().copied().map(Ref::new).collect(),
                instance_name: format!("{}:{}", node_idx.index(), op.name()),
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
                port_obs.create(id).declaration(event)
            }
        }
    }
}

// Create the following logical plan:
// Scan -> Project \                        Scan -> Project \
//                  -> Join -> Aggregate                    -> Join -> Aggregate -> Filter -> Udf \
// Scan -> Project /                        Scan -> Project /                                     -> Join -> Filter -> Udf -> Aggregate -> Sort -> Limit -> Output
//                                                                                Scan -> Project /
// Each Scan -> Project lowers to: FileSystemScan -> GpuDecode
fn make_logical_plan(query_id: Uuid, name: String) -> Plan<Logical> {
    fn add_scan_project_branch(plan: &mut Graph<Operator<Logical>, Edge, Directed>) -> NodeIndex {
        let scan = plan.add_node(Operator::new(Logical::Scan, vec![]));
        let project = plan.add_node(Operator::new(Logical::Project, vec![]));
        plan.add_edge(scan, project, Edge::new("out", "in"));
        project
    }

    fn add_join(
        plan: &mut Graph<Operator<Logical>, Edge, Directed>,
        left: NodeIndex,
        right: NodeIndex,
    ) -> NodeIndex {
        let join = plan.add_node(Operator::new(Logical::Join, vec![]));
        plan.add_edge(left, join, Edge::new("out", "left"));
        plan.add_edge(right, join, Edge::new("out", "right"));
        join
    }

    let mut dag = Graph::new();

    // Left branch: join scans A and B, then pre-aggregate
    let project_a = add_scan_project_branch(&mut dag);
    let project_b = add_scan_project_branch(&mut dag);
    let join_left = add_join(&mut dag, project_a, project_b);
    let agg_left = dag.add_node(Operator::new(Logical::Aggregate, vec![]));
    dag.add_edge(join_left, agg_left, Edge::new("out", "in"));

    // Right branch: join scans C and D, then pre-aggregate
    let project_c = add_scan_project_branch(&mut dag);
    let project_d = add_scan_project_branch(&mut dag);
    let join_right = add_join(&mut dag, project_c, project_d);
    let agg_right = dag.add_node(Operator::new(Logical::Aggregate, vec![]));
    dag.add_edge(join_right, agg_right, Edge::new("out", "in"));

    // Final join combining pre-aggregated sides
    let join_final = add_join(&mut dag, agg_left, agg_right);

    let aggregate = dag.add_node(Operator::new(Logical::Aggregate, vec![]));
    dag.add_edge(join_final, aggregate, Edge::new("out", "in"));

    let filter = dag.add_node(Operator::new(Logical::Filter, vec![]));
    dag.add_edge(aggregate, filter, Edge::new("out", "in"));

    let udf = dag.add_node(Operator::new(Logical::Udf, vec![]));
    dag.add_edge(filter, udf, Edge::new("out", "in"));

    // Late-stage dimension table lookup join
    let project_e = add_scan_project_branch(&mut dag);
    let join_lookup = add_join(&mut dag, udf, project_e);

    // Post-join processing before final sort
    let post_filter = dag.add_node(Operator::new(Logical::Filter, vec![]));
    dag.add_edge(join_lookup, post_filter, Edge::new("out", "in"));

    let post_udf = dag.add_node(Operator::new(Logical::Udf, vec![]));
    dag.add_edge(post_filter, post_udf, Edge::new("out", "in"));

    let post_aggregate = dag.add_node(Operator::new(Logical::Aggregate, vec![]));
    dag.add_edge(post_udf, post_aggregate, Edge::new("out", "in"));

    let sort = dag.add_node(Operator::new(Logical::Sort, vec![]));
    dag.add_edge(post_aggregate, sort, Edge::new("out", "in"));

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
            // Scan+Project lowers to FileSystemScan → GpuDecode
            if let Some(scan_edge) = logical
                .dag
                .edges_directed(logical_current_idx, Direction::Incoming)
                .find(|edge| logical.dag[edge.source()].kind == Logical::Scan)
            {
                let scan_op = &logical.dag[scan_edge.source()];
                let scan = physical.dag.add_node(Operator::new(
                    Physical::FileSystemScan,
                    vec![current_logical_op.id, scan_op.id],
                ));
                let decode = physical.dag.add_node(Operator::new(
                    Physical::GpuDecode,
                    vec![current_logical_op.id],
                ));
                physical.dag.add_edge(scan, decode, Edge::new("out", "in"));
                if let Some((target_node, target_port)) = physical_target_idx_port {
                    physical
                        .dag
                        .add_edge(decode, target_node, Edge::new(target_port, "in"));
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
                .add_edge(partition, local, Edge::new("out", "partitioned_in"));

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
        Logical::Aggregate | Logical::Filter | Logical::Udf | Logical::Sort => {
            let physical_kind = match current_logical_op.kind {
                Logical::Aggregate => Physical::Aggregate,
                Logical::Filter => Physical::Filter,
                Logical::Udf => Physical::Udf,
                Logical::Sort => Physical::Sort,
                _ => unreachable!(),
            };
            let node = physical
                .dag
                .add_node(Operator::new(physical_kind, vec![current_logical_op.id]));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical
                    .dag
                    .add_edge(node, target_node, Edge::new("out", target_port));
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
                Some((node, input_edge.weight().target.name)),
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

#[derive(Debug)]
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
    memory_handles: Vec<quent_stdlib::memory::MemoryHandle>,
    channel_handles: Vec<quent_stdlib::channel::ChannelHandle>,
    processor_handles: Vec<quent_stdlib::processor::ProcessorHandle>,
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
            memory_handles: Vec::new(),
            channel_handles: Vec::new(),
            processor_handles: Vec::new(),
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
        let worker_obs = context.worker_observer();
        worker_obs.create(self.id).init(worker::Init {
            parent_engine_id: Ref::new(parent_engine_id),
            instance_name: self.name.clone(),
        });

        let memory_obs = context.memory_observer();
        let channel_obs = context.channel_observer();
        let processor_obs = context.processor_observer();

        let mut host_memory = memory_obs.initializing(self.host_memory, "Host Memory", self.id);
        host_memory.operating(Some(0));
        self.memory_handles.push(host_memory);

        let mut storage = memory_obs.initializing(self.storage, "Storage", self.id);
        storage.operating(Some(0));
        self.memory_handles.push(storage);

        let mut storage_to_host = channel_obs.initializing(
            self.storage_to_host,
            "Storage -> Host",
            self.id,
            self.storage,
            self.host_memory,
        );
        storage_to_host.operating(None);
        self.channel_handles.push(storage_to_host);

        let mut host_to_storage = channel_obs.initializing(
            self.host_to_storage,
            "Host -> Storage",
            self.id,
            self.host_memory,
            self.storage,
        );
        host_to_storage.operating(None);
        self.channel_handles.push(host_to_storage);

        context
            .thread_pool_observer()
            .thread_pool(self.thread_pool, "Thread Pool", self.id);
        for (index, thread_id) in self.threads.iter().enumerate() {
            let mut thread = processor_obs.initializing(
                *thread_id,
                &format!("Thread {index}"),
                self.thread_pool,
            );
            thread.operating();
            self.processor_handles.push(thread);
        }

        for (index, gpu) in self.gpus.iter().enumerate() {
            context
                .gpu_observer()
                .gpu(gpu.id, &format!("GPU {index}"), self.id);

            let mut gpu_memory =
                memory_obs.initializing(gpu.memory, &format!("GPU {index} Memory"), gpu.id);
            gpu_memory.operating(Some(0));
            self.memory_handles.push(gpu_memory);

            let mut host_to_gpu = channel_obs.initializing(
                gpu.host_mem_to_gpu,
                &format!("Host -> GPU {index}"),
                gpu.id,
                self.host_memory,
                gpu.memory,
            );
            host_to_gpu.operating(None);
            self.channel_handles.push(host_to_gpu);

            let mut gpu_to_host = channel_obs.initializing(
                gpu.gpu_to_host_mem,
                &format!("GPU {index} -> Host"),
                gpu.id,
                gpu.memory,
                self.host_memory,
            );
            gpu_to_host.operating(None);
            self.channel_handles.push(gpu_to_host);
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
                    operator.id
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
        let mut task = context.task_observer().queueing(
            Uuid::now_v7(),
            &format!("task-{}", work.task_index),
            operator.id,
        );
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
        let thread_usage = || Some(usage(Ref::new(thread)));

        task.allocating(thread_usage());
        sleep_fixed(25);
        let reads_storage = operator.kind == Physical::FileSystemScan
            || input_batches.iter().any(|batch| batch.in_storage);
        if reads_storage {
            task.loading(
                thread_usage(),
                Some(usage((Ref::new(self.storage_to_host), input_bytes))),
                Some(usage((Ref::new(self.host_memory), input_bytes))),
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
                    Some(usage((
                        Ref::new(source_gpu.gpu_to_host_mem),
                        transfer_bytes,
                    ))),
                    Some(usage((Ref::new(self.host_memory), transfer_bytes))),
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
                    Some(usage((
                        Ref::new(target_gpu.host_mem_to_gpu),
                        host_to_gpu_bytes,
                    ))),
                    Some(usage((Ref::new(target_gpu.memory), host_to_gpu_bytes))),
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
                    Some(usage((
                        Ref::new(source_gpu.gpu_to_host_mem),
                        transfer_bytes,
                    ))),
                    Some(usage((Ref::new(self.host_memory), transfer_bytes))),
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
        let memory = gpu.map_or(self.host_memory, |device| device.memory);
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
            &operator.name(),
            input_bytes,
            thread_usage(),
            Some(usage((Ref::new(memory), working_bytes))),
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
                    Some(usage((Ref::new(self.host_to_storage), input_bytes))),
                );
                sleep_storage_io(input_bytes);
                task.allocating(thread_usage());
                task.computing(
                    "finalize output",
                    0,
                    thread_usage(),
                    Some(usage((Ref::new(self.host_memory), 0))),
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
                let link = engine.network_links[&(self.id, *other)];
                let network_bytes = output_bytes
                    .saturating_mul(engine.workers.len().saturating_sub(1) as u64)
                    / engine.workers.len() as u64;
                task.sending(thread_usage(), Some(usage((Ref::new(link), network_bytes))));
                sleep_network(network_bytes);
                task.queueing("network completion", operator.id);
                task.allocating(thread_usage());
                task.computing(
                    "finalize shuffle",
                    0,
                    thread_usage(),
                    Some(usage((Ref::new(memory), 0))),
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
        let physical_plan = simulate_planning(logical_plan);
        physical_plan.declare(context, Some(self.id));
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

        let op_obs = context.operator_observer();
        for &node in &nodes {
            let operator = &physical_plan.dag[node];
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
            op_obs.create(operator.id).statistics(operator::Statistics {
                custom_attributes: attributes.into(),
            });
        }

        let port_obs = context.port_observer();
        for edge in physical_plan.dag.edge_references() {
            let source = &physical_plan.dag[edge.source()];
            let attributes = || {
                vec![
                    DynamicAttribute::u64("bytes", source.bytes_out.load(Ordering::Relaxed)),
                    DynamicAttribute::u64("rows", source.rows_out.load(Ordering::Relaxed)),
                ]
                .into()
            };
            port_obs
                .create(edge.weight().source.id)
                .statistics(port::Statistics {
                    custom_attributes: attributes(),
                });
            port_obs
                .create(edge.weight().target.id)
                .statistics(port::Statistics {
                    custom_attributes: attributes(),
                });
        }
    }

    fn shut_down(&mut self, context: &SimulatorContext) {
        for handle in &mut self.memory_handles {
            handle.finalizing();
            handle.exit();
            sleep_fixed(25);
        }
        for handle in &mut self.channel_handles {
            handle.finalizing();
            handle.exit();
            sleep_fixed(25);
        }
        for handle in &mut self.processor_handles {
            handle.finalizing();
            handle.exit();
        }
        context.worker_observer().create(self.id).exit(worker::Exit);
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
            id: ENGINE_ID,
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
        num_gpus: usize,
    ) {
        info!("Simulating Engine {}", self.id);
        let engine_obs = context.engine_observer();
        engine_obs.create(self.id).init(engine::Init {
            instance_name: Some(format!("holodeck-{:04x}", rng().random::<u32>())),
            implementation: EngineImplementationAttributes {
                name: Some("Simulator".into()),
                version: Some("vibe".into()),
                custom_attributes: Default::default(),
            },
        });

        // Workers
        let worker_ids = std::iter::repeat_with(Uuid::now_v7)
            .take(num_workers)
            .collect::<Vec<_>>();

        for (worker_index, worker_id) in worker_ids.iter().enumerate() {
            let mut worker = Worker::new(
                *worker_id,
                format!("worker-{worker_index}"),
                num_threads,
                num_gpus,
                nvtx,
            );
            worker.spawn(context, self.id);
            self.workers.insert(*worker_id, worker);
        }

        // Engine-wide resources
        // Create a fully connected bidirectional network of workers
        context
            .network_observer()
            .network(self.network, "Network", self.id);
        let channel_obs = context.channel_observer();
        for worker_index in 0..worker_ids.len() {
            for other_worker_index in worker_index + 1..worker_ids.len() {
                let worker_id = worker_ids[worker_index];
                let other_worker_id = worker_ids[other_worker_index];
                let up_link_id = Uuid::now_v7();
                let mut up_link = channel_obs.initializing(
                    up_link_id,
                    &format!("worker {worker_index} -> {other_worker_index}"),
                    self.network,
                    self.workers[&worker_id].host_memory,
                    self.workers[&other_worker_id].host_memory,
                );
                up_link.operating(None);
                self.network_link_handles.push(up_link);

                let down_link_id = Uuid::now_v7();
                let mut down_link = channel_obs.initializing(
                    down_link_id,
                    &format!("worker {other_worker_index} -> {worker_index}"),
                    self.network,
                    self.workers[&other_worker_id].host_memory,
                    self.workers[&worker_id].host_memory,
                );
                down_link.operating(None);
                self.network_link_handles.push(down_link);

                self.network_links
                    .insert((worker_id, other_worker_id), up_link_id);
                self.network_links
                    .insert((other_worker_id, worker_id), down_link_id);
            }
        }
    }

    fn shut_down(&mut self, context: &SimulatorContext) {
        for handle in &mut self.network_link_handles {
            handle.finalizing();
            handle.exit();
        }

        // Tear down workers
        for worker in self.workers.values_mut() {
            worker.shut_down(context);
        }

        context.engine_observer().create(self.id).exit(engine::Exit);
        info!("Simulated engine shut down.")
    }
}

/// Controls the amount of telemetry emitted by a simulator run.
#[derive(Clone, Copy, Debug)]
pub struct SimulationConfig {
    /// Number of query groups.
    pub num_query_groups: usize,
    /// Number of queries in each query group.
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
            num_query_groups: 1,
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
    simulate_with_nvtx(context, config, nvtx);
}

fn simulate_with_nvtx(context: SimulatorContext, config: SimulationConfig, nvtx: NvtxCapture) {
    let nvtx_workload = NvtxWorkload::from_config(config);
    nvtx.declare_schema();
    let main_thread = nvtx.name_thread("main");
    let simulation_resource = nvtx.resource(nvtx.domain_at(0), "simulated CUDA execution context");

    let mut engine = Engine::new();
    engine.spawn(
        &context,
        &nvtx,
        config.num_workers,
        config.num_threads,
        config.num_gpus,
    );

    for (query_group_index, query_group_id) in std::iter::repeat_with(Uuid::now_v7)
        .take(config.num_query_groups)
        .enumerate()
    {
        let query_group_obs = context.query_group_observer();
        query_group_obs.declaration(
            query_group_id,
            query_group::Declaration {
                engine_id: engine.id,
                instance_name: format!("Simulated workload (run {query_group_index})"),
            },
        );

        // "Run" the specified number of queries, sequentially for now.
        for query_index in 0..config.num_queries {
            let total = config.num_query_groups * config.num_queries;
            let done = query_group_index * config.num_queries + query_index;
            let query_id = Uuid::from_u128(QUERY_ID_BASE + done as u128);
            info!("{}% ({}/{})", done * 100 / total, done, total);
            const QUERY_NUMBERS: &[u32] = &[42, 1337, 7, 404, 256, 99, 13, 1024, 69, 314];
            let n = QUERY_NUMBERS[query_index % QUERY_NUMBERS.len()];
            let query_name = format!("Q{n}");
            let query_obs = context.query_observer();
            let mut query = query_obs.init(query_id, &query_name, Ref::new(query_group_id));
            query.planning();
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
            query.executing();
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

            query.exit();
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
        num_query_groups: args.num_query_groups,
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
        None => SimulatorContext::try_new(quent_model::Noop)?,
    };
    let nvtx = match exporter.as_ref() {
        Some(provider) => NvtxCapture::try_new(context.id(), provider, nvtx_layout)?,
        None => NvtxCapture::noop(context.id(), nvtx_layout),
    };
    simulate_with_nvtx(context, config, nvtx);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
