use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender};

use clap::Parser;
use petgraph::{Directed, Direction, Graph, graph::NodeIndex, visit::EdgeRef};
use quent_attributes::Attribute;
use quent_events::resource::{self, channel, memory};
use quent_exporter::{
    CollectorExporterOptions, ExporterOptions, MsgpackExporterOptions, NdjsonExporterOptions,
    PostcardExporterOptions,
};
use quent_query_engine_events::{
    engine::{self, EngineImplementationAttributes},
    operator, plan, port, query, query_group, worker,
};
use quent_simulator_events::{data_batch, task};
use quent_simulator_instrumentation::SimulatorContext;
use rand::{Rng, rng};
use tracing::info;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "simulator")]
#[command(about = "Emits simulated query engine telemetry", long_about = None)]
struct Args {
    /// Number of query groups
    #[arg(long, default_value_t = 2)]
    num_query_groups: usize,

    /// Number of queries per query group
    #[arg(long, default_value_t = 4)]
    num_queries: usize,

    /// Number of tasks per operator
    #[arg(long, default_value_t = 1024)]
    num_tasks: usize,

    /// Number of workers
    #[arg(long, default_value_t = 4)]
    num_workers: usize,

    /// Number of threads per worker thread pool
    #[arg(long, default_value_t = 4)]
    num_threads: usize,

    /// Number of GPUs per worker
    #[arg(long, default_value_t = 2)]
    num_gpus: usize,

    /// Exporter format:
    /// - collector: send events to a collector service over gRPC.
    /// - postcard: binary format, NOT self-describing, most performant.
    /// - messagepack: binary self-describing format.
    /// - ndjson: newline-delimited JSON files (human readable).
    #[arg(long, default_value = "collector")]
    exporter: String,

    /// Collector address (when --exporter is collector)
    /// Overridden by the QUENT_COLLECTOR_ADDRESS environment variable if set.
    #[arg(
        long,
        default_value = "http://localhost:7836",
        env = "QUENT_COLLECTOR_ADDRESS"
    )]
    collector_address: String,
}

fn initialize_tracing() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();
}

fn sleep_fixed(micros: u64) {
    std::thread::sleep(Duration::from_micros(micros));
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

/// Sleep proportional to the number of bytes being processed.
fn sleep_proportional(bytes: u64) {
    let mib = (bytes / (1024 * 1024)).max(1);
    let micros = 5 + mib / 4;
    std::thread::sleep(Duration::from_micros(micros));
}

/// Occasionally a bit slower (1% of the time), otherwise proportional.
fn sleep_sometimes_slow(bytes: u64) {
    let mib = (bytes / (1024 * 1024)).max(1);
    std::thread::sleep(Duration::from_micros(if rng().random_ratio(1, 100) {
        10 + mib
    } else {
        5 + mib / 4
    }));
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

/// A work item dispatched by the scheduler to a pool thread.
struct WorkItem<'a> {
    operator_node: NodeIndex,
    operator: &'a Operator<Physical>,
    /// Input batches (empty for scan operators which produce their own).
    input_batches: Vec<Batch>,
    /// Senders for the operator's outgoing edges in the DAG.
    output_senders: Vec<&'a Sender<Batch>>,
    /// Task index for naming.
    task_index: u64,
}

/// Returns true if this operator kind is a barrier — it must wait for all
/// upstream batches before processing.
fn is_barrier(kind: Physical) -> bool {
    matches!(
        kind,
        Physical::JoinPartition | Physical::Aggregate | Physical::Sort
    )
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

        plan_obs.plan(
            self.id,
            plan::PlanEvent {
                instance_name: self.name.clone(),
                parent: self
                    .parent_plan_id
                    .map_or(plan::PlanParent::Query(self.query_id), |parent_id| {
                        plan::PlanParent::Plan(parent_id)
                    }),
                worker_id,
                edges: self
                    .dag
                    .edge_references()
                    .map(|edge| plan::Edge {
                        source: edge.weight().source.id,
                        target: edge.weight().target.id,
                    })
                    .collect(),
            },
        );

        // Declare all operators
        for node_idx in self.dag.node_indices() {
            let op = &self.dag[node_idx];
            operator_obs.operator(
                op.id,
                operator::Declaration {
                    plan_id: self.id,
                    parent_operator_ids: op.parents.clone(),
                    instance_name: format!("{}:{}", node_idx.index(), op.name()),
                    type_name: op.name(),
                    custom_attributes: vec![],
                },
            );

            // Declare operator ports
            for (id, event) in self
                .dag
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .map(|edge| {
                    (
                        edge.weight().target.id,
                        port::Declaration {
                            operator_id: op.id,
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
                                    operator_id: op.id,
                                    instance_name: edge.weight().source.name.to_string(),
                                },
                            )
                        }),
                )
            {
                port_obs.port(id, event)
            }
        }
    }
}

// Create the following logical plan:
// Scan -> Project \                        Scan -> Project \
//                  -> Join -> Aggregate                    -> Join -> Aggregate -> Filter -> Udf \
// Scan -> Project /                        Scan -> Project /                                     -> Join -> Sort -> Limit -> Output
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

    let sort = dag.add_node(Operator::new(Logical::Sort, vec![]));
    dag.add_edge(join_lookup, sort, Edge::new("out", "in"));

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

/// Capacity of GPU memory per device in bytes (4 GiB).
const GPU_MEMORY_CAPACITY: u64 = 4 * 1024 * 1024 * 1024;
/// Spill GPU→host when GPU memory usage exceeds 80% of capacity.
const GPU_MEMORY_SPILL_THRESHOLD: f64 = 0.80;

#[derive(Debug)]
struct Gpu {
    id: Uuid,
    memory: Uuid,
    compute: Uuid,
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
            compute: Uuid::now_v7(),
            host_mem_to_gpu: Uuid::now_v7(),
            gpu_to_host_mem: Uuid::now_v7(),
            memory_used: AtomicU64::new(0),
        }
    }
}

#[derive(Clone, Debug)]
struct Batch {
    id: Uuid,
    bytes: u64,
    rows: u64,
    /// Index into the worker's `gpus` vec if this batch is currently on a GPU.
    /// `None` means the batch is in host memory (or in storage if `in_storage` is true).
    gpu_index: Option<usize>,
    /// Batch has been spilled to storage; memory is not tracked on host or GPU.
    in_storage: bool,
}

/// Capacity of host memory per worker in bytes (16 GiB).
const HOST_MEMORY_CAPACITY: u64 = 16 * 1024 * 1024 * 1024;
/// Spill threshold: spill when host memory usage exceeds 75% of capacity.
const HOST_MEMORY_SPILL_THRESHOLD: f64 = 0.75;

#[derive(Debug)]
struct Worker {
    id: Uuid,
    name: String,
    host_group: Uuid,
    host_memory: Uuid,
    /// Tracks current host memory usage in bytes for spill decisions.
    host_memory_used: AtomicU64,
    thread_pool: Uuid,
    storage_group: Uuid,
    storage: Uuid,
    storage_to_host: Uuid,
    host_to_storage: Uuid,
    threads: Vec<Uuid>,
    gpus: Vec<Gpu>,
}

impl Worker {
    fn new(id: Uuid, name: String, num_threads: usize, num_gpus: usize) -> Self {
        Self {
            id,
            name,
            host_group: Uuid::now_v7(),
            host_memory: Uuid::now_v7(),
            host_memory_used: AtomicU64::new(0),
            thread_pool: Uuid::now_v7(),
            storage_group: Uuid::now_v7(),
            storage: Uuid::now_v7(),
            storage_to_host: Uuid::now_v7(),
            host_to_storage: Uuid::now_v7(),
            threads: std::iter::repeat_with(Uuid::now_v7)
                .take(num_threads)
                .collect(),
            gpus: std::iter::repeat_with(Gpu::new).take(num_gpus).collect(),
        }
    }

    fn spawn(&self, context: &SimulatorContext, parent_engine_id: Uuid) {
        let worker_obs = context.worker_observer();
        let resource_group_obs = context.resource_group_observer();
        let memory_obs = context.memory_resource_observer();
        let channel_obs = context.channel_resource_observer();
        let processor_obs = context.processor_resource_observer();

        worker_obs.init(
            self.id,
            worker::Init {
                parent_engine_id,
                instance_name: self.name.clone(),
            },
        );

        // Host group: host memory + thread pool
        resource_group_obs.group(
            self.host_group,
            resource::GroupEvent {
                type_name: "host".to_string(),
                instance_name: "Host".to_string(),
                parent_group_id: Some(self.id),
            },
        );

        memory_obs.init(
            self.host_memory,
            memory::Init {
                resource: resource::Resource {
                    type_name: "host_memory".to_string(),
                    instance_name: "Host Memory".to_string(),
                    parent_group_id: self.host_group,
                },
            },
        );
        memory_obs.operating(self.host_memory, Default::default());

        resource_group_obs.group(
            self.thread_pool,
            resource::GroupEvent {
                type_name: "thread_pool".to_string(),
                instance_name: "Thread Pool".to_string(),
                parent_group_id: Some(self.host_group),
            },
        );

        // Storage group: storage + IO channels
        resource_group_obs.group(
            self.storage_group,
            resource::GroupEvent {
                type_name: "storage".to_string(),
                instance_name: "Storage".to_string(),
                parent_group_id: Some(self.id),
            },
        );

        memory_obs.init(
            self.storage,
            memory::Init {
                resource: resource::Resource {
                    type_name: "storage".to_string(),
                    instance_name: "Storage".to_string(),
                    parent_group_id: self.storage_group,
                },
            },
        );
        memory_obs.operating(self.storage, Default::default());

        channel_obs.init(
            self.storage_to_host,
            channel::Init {
                resource: resource::Resource {
                    type_name: "storage_to_host".to_string(),
                    instance_name: "S2H".to_string(),
                    parent_group_id: self.storage_group,
                },
                source_id: self.storage,
                target_id: self.host_memory,
            },
        );
        channel_obs.operating(self.storage_to_host, Default::default());

        channel_obs.init(
            self.host_to_storage,
            channel::Init {
                resource: resource::Resource {
                    type_name: "host_to_storage".to_string(),
                    instance_name: "H2S".to_string(),
                    parent_group_id: self.storage_group,
                },
                source_id: self.host_memory,
                target_id: self.storage,
            },
        );
        channel_obs.operating(self.host_to_storage, Default::default());
        for (index, thread_id) in self.threads.iter().enumerate() {
            processor_obs.init(
                *thread_id,
                resource::processor::Init {
                    resource: resource::Resource {
                        type_name: "thread".to_string(),
                        instance_name: format!("Thread {index}"),
                        parent_group_id: self.thread_pool,
                    },
                },
            );
            processor_obs.operating(*thread_id, Default::default());
        }

        // GPUs
        for (index, gpu) in self.gpus.iter().enumerate() {
            resource_group_obs.group(
                gpu.id,
                resource::GroupEvent {
                    type_name: "gpu".to_string(),
                    instance_name: format!("GPU {index}"),
                    parent_group_id: Some(self.id),
                },
            );

            memory_obs.init(
                gpu.memory,
                memory::Init {
                    resource: resource::Resource {
                        type_name: "gpu_memory".to_string(),
                        instance_name: format!("GPU {index} Memory"),
                        parent_group_id: gpu.id,
                    },
                },
            );
            memory_obs.operating(gpu.memory, Default::default());

            processor_obs.init(
                gpu.compute,
                resource::processor::Init {
                    resource: resource::Resource {
                        type_name: "gpu_compute".to_string(),
                        instance_name: format!("GPU {index} Compute"),
                        parent_group_id: gpu.id,
                    },
                },
            );
            processor_obs.operating(gpu.compute, Default::default());
        }

        // Per-GPU H2D/D2H channels live under each GPU's resource group.
        for (index, gpu) in self.gpus.iter().enumerate() {
            channel_obs.init(
                gpu.host_mem_to_gpu,
                channel::Init {
                    resource: resource::Resource {
                        type_name: "h2d".to_string(),
                        instance_name: format!("H2D GPU {index}"),
                        parent_group_id: gpu.id,
                    },
                    source_id: self.host_memory,
                    target_id: gpu.memory,
                },
            );
            channel_obs.operating(gpu.host_mem_to_gpu, Default::default());

            channel_obs.init(
                gpu.gpu_to_host_mem,
                channel::Init {
                    resource: resource::Resource {
                        type_name: "d2h".to_string(),
                        instance_name: format!("D2H GPU {index}"),
                        parent_group_id: gpu.id,
                    },
                    source_id: gpu.memory,
                    target_id: self.host_memory,
                },
            );
            channel_obs.operating(gpu.gpu_to_host_mem, Default::default());
        }
    }

    /// Process a single work item dispatched by the scheduler.
    fn process_work_item(
        &self,
        context: &SimulatorContext,
        engine: &Engine,
        work: &WorkItem,
        thread: Uuid,
    ) {
        let obs = context.task_observer();
        let batch_obs = context.data_batch_observer();
        let operator = work.operator;

        let task_id = Uuid::now_v7();
        obs.task_queueing(
            task_id,
            task::Queueing {
                operator_id: operator.id,
                instance_name: format!("task-{}", work.task_index),
            },
        );
        sleep_fixed(25);

        // FileSystemScan: create a batch from storage (heavy I/O).
        // Each scan has a different size distribution derived from its
        // operator ID to simulate data skew across input tables.
        let mut input_batches = if operator.kind == Physical::FileSystemScan {
            let batch_id = Uuid::now_v7();
            let skew = (operator.id.as_bytes()[0] % 5) as u64 + 1; // 1-5x scale
            let base_bytes = rng().random_range(8..64) * 1024 * 1024;
            let batch_bytes = base_bytes * skew;
            let batch_rows = rng().random_range(1024..16384) * skew;
            batch_obs.init(
                batch_id,
                data_batch::Init {
                    operator_id: operator.id,
                },
            );
            // Read external files into host memory (no storage resource
            // usage — input files are externally managed).
            batch_obs.loading_to_host_memory(
                batch_id,
                data_batch::LoadingToHostMemory {
                    use_storage_to_host: self.storage_to_host,
                    use_storage_to_host_bytes: batch_bytes,
                },
            );
            sleep_proportional(batch_bytes * 5);
            self.host_memory_used
                .fetch_add(batch_bytes, Ordering::Relaxed);
            batch_obs.in_host_memory(
                batch_id,
                data_batch::InHostMemory {
                    use_host_memory: self.host_memory,
                    use_host_memory_bytes: batch_bytes,
                },
            );
            vec![Batch {
                id: batch_id,
                bytes: batch_bytes,
                rows: batch_rows,
                gpu_index: None,
                in_storage: false,
            }]
        } else {
            work.input_batches.clone()
        };

        // Derive task resource usage from input batch size.
        // For barrier operators, use average batch size for working memory
        // since they stream through batches rather than holding all at once.
        let total_batch_bytes: u64 = input_batches.iter().map(|b| b.bytes).sum();
        let batch_bytes = if input_batches.len() > 1 {
            total_batch_bytes / input_batches.len() as u64
        } else {
            total_batch_bytes
        };
        // Working memory scales with operator complexity.
        // JoinLocal needs hash table + build/probe buffers (3-6x).
        // Sort needs merge buffers (2-4x). Others 1-2x.
        let mem_multiplier = match operator.kind {
            Physical::JoinLocal => rng().random_range(3..7),
            Physical::Aggregate => rng().random_range(2..5),
            Physical::Sort => rng().random_range(2..5),
            _ => rng().random_range(1..3),
        };
        let working_memory_bytes = batch_bytes * mem_multiplier;

        // Determine operator behavior based on kind.
        let use_gpu = operator.kind != Physical::FileSystemScan && !self.gpus.is_empty();
        let send = operator.kind == Physical::JoinPartition;

        // Track working memory on the appropriate resource.
        // GPU operators use GPU memory for their scratch space; CPU-only
        // operators use host memory.
        if !use_gpu {
            self.host_memory_used
                .fetch_add(working_memory_bytes, Ordering::Relaxed);
        }

        // Spill when host memory exceeds threshold and operator supports it.
        let host_pressure = self.host_memory_used.load(Ordering::Relaxed) as f64
            / HOST_MEMORY_CAPACITY as f64;
        let spill = host_pressure > HOST_MEMORY_SPILL_THRESHOLD;

        obs.task_allocating_memory(task_id, task::Allocating { use_thread: thread });
        sleep_fixed(2);

        // Spill batches to storage under pressure.
        if spill {
            obs.task_spilling(
                task_id,
                task::Spilling {
                    use_thread: thread,
                },
            );
            // Release memory and spill to storage.
            for batch in &mut input_batches {
                if let Some(gi) = batch.gpu_index {
                    saturating_sub(&self.gpus[gi].memory_used, batch.bytes);
                    batch.gpu_index = None;
                } else if !batch.in_storage {
                    saturating_sub(&self.host_memory_used, batch.bytes);
                }
                batch.in_storage = true;
                batch_obs.spilling_to_storage(
                    batch.id,
                    data_batch::SpillingToStorage {
                        use_host_to_storage: self.host_to_storage,
                        use_host_to_storage_bytes: batch.bytes,
                    },
                );
                sleep_proportional(batch.bytes);
                batch_obs.in_storage(
                    batch.id,
                    data_batch::InStorage {
                        use_storage: self.storage,
                        use_storage_bytes: batch.bytes,
                    },
                );
            }
            sleep_sometimes_slow(batch_bytes);
        }

        // Loading (scan already loaded above; this is for materialization).
        if operator.kind != Physical::FileSystemScan && rng().random_bool(0.2) {
            obs.task_loading(
                task_id,
                task::Loading {
                    use_thread: thread,
                    use_host_memory: self.host_memory,
                    use_host_memory_bytes: working_memory_bytes,
                },
            );
            sleep_sometimes_slow(batch_bytes);
        }

        // Pick GPU if applicable. Prefer the GPU the first batch is already on.
        let gpu_index = if use_gpu {
            input_batches
                .first()
                .and_then(|b| b.gpu_index)
                .unwrap_or_else(|| rng().random_range(0..self.gpus.len()))
                .into()
        } else {
            None
        };
        let gpu = gpu_index.map(|i: usize| &self.gpus[i]);

        // Move input batch(es) to GPU memory before compute.
        // For pipeline operators (single batch), transfer the batch.
        // For barrier operators (many batches), skip bulk transfer —
        // the operator streams through data with bounded GPU working memory.
        if let Some(gpu) = gpu && !is_barrier(operator.kind) {
            for batch in &mut input_batches {
                if batch.gpu_index.is_none() {
                    // Reload from storage if needed.
                    if batch.in_storage {
                        batch_obs.loading_to_host_memory(
                            batch.id,
                            data_batch::LoadingToHostMemory {
                                use_storage_to_host: self.storage_to_host,
                                use_storage_to_host_bytes: batch.bytes,
                            },
                        );
                        sleep_proportional(batch.bytes);
                        self.host_memory_used
                            .fetch_add(batch.bytes, Ordering::Relaxed);
                        batch_obs.in_host_memory(
                            batch.id,
                            data_batch::InHostMemory {
                                use_host_memory: self.host_memory,
                                use_host_memory_bytes: batch.bytes,
                            },
                        );
                        batch.in_storage = false;
                    }
                    // Transfer host → GPU.
                    batch_obs.loading_to_gpu_memory(
                        batch.id,
                        data_batch::LoadingToGpuMemory {
                            use_host_mem_to_gpu: gpu.host_mem_to_gpu,
                            use_host_mem_to_gpu_bytes: batch.bytes,
                        },
                    );
                    sleep_proportional(batch.bytes);
                    saturating_sub(&self.host_memory_used, batch.bytes);
                    gpu.memory_used.fetch_add(batch.bytes, Ordering::Relaxed);
                    batch_obs.in_gpu_memory(
                        batch.id,
                        data_batch::InGpuMemory {
                            use_gpu_memory: gpu.memory,
                            use_gpu_memory_bytes: batch.bytes,
                        },
                    );
                    batch.gpu_index = gpu_index;
                }
            }
        }

        // Compute time scales with operator complexity and GPU availability.
        let compute_multiplier = match operator.kind {
            Physical::JoinLocal => 4,
            Physical::Udf => 3,
            Physical::Aggregate => 3,
            Physical::GpuDecode => 2,
            Physical::Sort => 2,
            Physical::JoinPartition => 2,
            _ => 1,
        };
        let gpu_multiplier: u64 = if gpu.is_some() {
            match operator.kind {
                Physical::JoinLocal => 6,
                Physical::Udf => 5,
                Physical::GpuDecode => 4,
                Physical::Sort => 3,
                Physical::JoinPartition => 2,
                _ => 1,
            }
        } else {
            0
        };
        // GPU working memory for scratch buffers, intermediate results, etc.
        let gpu_working_memory_bytes = gpu.map_or(0, |_| working_memory_bytes);
        // Track GPU working memory pressure during compute.
        if let Some(gpu) = gpu {
            gpu.memory_used
                .fetch_add(gpu_working_memory_bytes, Ordering::Relaxed);
        }
        obs.task_computing(
            task_id,
            task::Computing {
                use_thread: thread,
                use_host_memory: self.host_memory,
                use_host_memory_bytes: if use_gpu { 0 } else { working_memory_bytes },
                use_gpu_compute: gpu.map_or(Uuid::nil(), |g| g.compute),
                use_gpu_memory: gpu.map_or(Uuid::nil(), |g| g.memory),
                use_gpu_memory_bytes: gpu_working_memory_bytes,
            },
        );
        // For operators that consume input (not scan/decode/output which
        // forward or write batches directly), release each input batch's
        // memory during compute so the decrease is gradual.
        let multiplier = compute_multiplier + gpu_multiplier;
        let consumes_input = !matches!(
            operator.kind,
            Physical::FileSystemScan | Physical::GpuDecode | Physical::Output
        );
        if consumes_input {
            for batch in &input_batches {
                sleep_proportional(batch.bytes * multiplier);
                if let Some(gi) = batch.gpu_index {
                    saturating_sub(&self.gpus[gi].memory_used, batch.bytes);
                } else if !batch.in_storage {
                    saturating_sub(&self.host_memory_used, batch.bytes);
                }
                batch_obs.exit(batch.id);
            }
        } else {
            sleep_proportional(total_batch_bytes * multiplier);
        }
        // Release GPU working memory after compute.
        if let Some(gpu) = gpu {
            saturating_sub(&gpu.memory_used, gpu_working_memory_bytes);
        }

        // Only spill GPU→host when GPU memory exceeds threshold.
        // Skip for operators that already consumed and freed their input
        // batches during the compute phase above.
        if !consumes_input {
            if let Some(gpu) = gpu {
                for batch in &mut input_batches {
                    let gpu_pressure = gpu.memory_used.load(Ordering::Relaxed) as f64
                        / GPU_MEMORY_CAPACITY as f64;
                    if gpu_pressure > GPU_MEMORY_SPILL_THRESHOLD {
                        batch_obs.spilling_to_host_memory(
                            batch.id,
                            data_batch::SpillingToHostMemory {
                                use_gpu_to_host_mem: gpu.gpu_to_host_mem,
                                use_gpu_to_host_mem_bytes: batch.bytes,
                            },
                        );
                        sleep_proportional(batch.bytes);
                        saturating_sub(&gpu.memory_used, batch.bytes);
                        self.host_memory_used
                            .fetch_add(batch.bytes, Ordering::Relaxed);
                        batch_obs.in_host_memory(
                            batch.id,
                            data_batch::InHostMemory {
                                use_host_memory: self.host_memory,
                                use_host_memory_bytes: batch.bytes,
                            },
                        );
                        batch.gpu_index = None;

                        // Cascade: spill host→storage if host is now over threshold.
                        let hp = self.host_memory_used.load(Ordering::Relaxed) as f64
                            / HOST_MEMORY_CAPACITY as f64;
                        if hp > HOST_MEMORY_SPILL_THRESHOLD {
                            saturating_sub(&self.host_memory_used, batch.bytes);
                            batch_obs.spilling_to_storage(
                                batch.id,
                                data_batch::SpillingToStorage {
                                    use_host_to_storage: self.host_to_storage,
                                    use_host_to_storage_bytes: batch.bytes,
                                },
                            );
                            sleep_proportional(batch.bytes);
                            batch_obs.in_storage(
                                batch.id,
                                data_batch::InStorage {
                                    use_storage: self.storage,
                                    use_storage_bytes: batch.bytes,
                                },
                            );
                            batch.in_storage = true;
                        }
                    }
                    // else: batch stays on GPU for the next operator.
                }
            }
        }

        // Release working memory from the appropriate resource.
        if !use_gpu {
            saturating_sub(&self.host_memory_used, working_memory_bytes);
        }

        // Produce output batches and send downstream.
        match operator.kind {
            Physical::FileSystemScan | Physical::GpuDecode => {
                // Scan and decode pass through batches as-is.
                for batch in input_batches {
                    operator.batches_out.fetch_add(1, Ordering::Relaxed);
                    operator.bytes_out.fetch_add(batch.bytes, Ordering::Relaxed);
                    operator.rows_out.fetch_add(batch.rows, Ordering::Relaxed);
                    for sender in &work.output_senders {
                        let _ = sender.send(batch.clone());
                    }
                }
            }
            Physical::Output => {
                for batch in input_batches {
                    operator.batches_in.fetch_add(1, Ordering::Relaxed);
                    operator.bytes_in.fetch_add(batch.bytes, Ordering::Relaxed);
                    operator.rows_in.fetch_add(batch.rows, Ordering::Relaxed);
                    // GPU→host if needed before writing to storage.
                    if let Some(gi) = batch.gpu_index {
                        let gpu = &self.gpus[gi];
                        saturating_sub(&gpu.memory_used, batch.bytes);
                        self.host_memory_used
                            .fetch_add(batch.bytes, Ordering::Relaxed);
                        batch_obs.spilling_to_host_memory(
                            batch.id,
                            data_batch::SpillingToHostMemory {
                                use_gpu_to_host_mem: gpu.gpu_to_host_mem,
                                use_gpu_to_host_mem_bytes: batch.bytes,
                            },
                        );
                        sleep_proportional(batch.bytes);
                        batch_obs.in_host_memory(
                            batch.id,
                            data_batch::InHostMemory {
                                use_host_memory: self.host_memory,
                                use_host_memory_bytes: batch.bytes,
                            },
                        );
                    }
                    // Write result to storage.
                    if !batch.in_storage {
                        saturating_sub(&self.host_memory_used, batch.bytes);
                    }
                    batch_obs.spilling_to_storage(
                        batch.id,
                        data_batch::SpillingToStorage {
                            use_host_to_storage: self.host_to_storage,
                            use_host_to_storage_bytes: batch.bytes,
                        },
                    );
                    sleep_proportional(batch.bytes);
                    batch_obs.in_storage(
                        batch.id,
                        data_batch::InStorage {
                            use_storage: self.storage,
                            use_storage_bytes: batch.bytes,
                        },
                    );
                    batch_obs.exit(batch.id);
                }
            }
            _ => {
                // For barrier operators that use GPU, output goes to the
                // selected GPU even though input wasn't bulk-transferred.
                let last_gpu_index: Option<usize> = if is_barrier(operator.kind) {
                    gpu_index
                } else {
                    input_batches.last().and_then(|b| b.gpu_index)
                };
                // Track input stats (memory already released during compute).
                for batch in &input_batches {
                    operator.batches_in.fetch_add(1, Ordering::Relaxed);
                    operator.bytes_in.fetch_add(batch.bytes, Ordering::Relaxed);
                    operator.rows_in.fetch_add(batch.rows, Ordering::Relaxed);
                }

                // Produce one output batch per input batch, with size
                // transformation applied per-batch.
                for in_batch in &input_batches {
                    let (output_bytes, output_rows) = match operator.kind {
                        Physical::JoinLocal => {
                            let factor = rng().random_range(1..4);
                            (factor * in_batch.bytes, factor * in_batch.rows)
                        }
                        Physical::Aggregate => {
                            let denom = rng().random_range(5..15);
                            (
                                in_batch.bytes / denom,
                                in_batch.rows / denom.max(1),
                            )
                        }
                        Physical::Filter => {
                            let keep = rng().random_range(60..80);
                            (
                                in_batch.bytes * keep / 100,
                                in_batch.rows * keep / 100,
                            )
                        }
                        Physical::Udf => (in_batch.bytes, in_batch.rows),
                        Physical::Limit => {
                            let emitted_so_far =
                                operator.rows_out.load(Ordering::Relaxed);
                            let remaining =
                                42u64.saturating_sub(emitted_so_far);
                            if remaining == 0 {
                                (0, 0)
                            } else {
                                let limit_rows = remaining.min(in_batch.rows);
                                let fraction = if in_batch.rows > 0 {
                                    limit_rows as f64 / in_batch.rows as f64
                                } else {
                                    1.0
                                };
                                (
                                    (in_batch.bytes as f64 * fraction) as u64,
                                    limit_rows,
                                )
                            }
                        }
                        _ => {
                            let denom = rng().random_range(1..3);
                            (
                                in_batch.bytes / denom,
                                in_batch.rows / denom,
                            )
                        }
                    };

                    if output_rows == 0 {
                        continue;
                    }

                    // Split large outputs into chunks so each piece
                    // can be individually spilled under memory pressure.
                    const MAX_CHUNK_BYTES: u64 = 64 * 1024 * 1024;
                    let num_chunks = (output_bytes / MAX_CHUNK_BYTES).max(1);
                    let chunk_bytes = output_bytes / num_chunks;
                    let chunk_rows = output_rows / num_chunks;

                    for _chunk in 0..num_chunks {
                        operator.batches_out.fetch_add(1, Ordering::Relaxed);
                        operator
                            .bytes_out
                            .fetch_add(chunk_bytes, Ordering::Relaxed);
                        operator
                            .rows_out
                            .fetch_add(chunk_rows, Ordering::Relaxed);

                        // Network shuffle per output chunk.
                        if send {
                            let other_workers =
                                engine.workers.keys().filter(|w| **w != self.id);
                            for other in other_workers {
                                let link = *engine
                                    .network_links
                                    .get(&(self.id, *other))
                                    .unwrap();
                                obs.task_sending(
                                    task_id,
                                    task::Sending {
                                        use_thread: thread,
                                        use_link: link,
                                        use_link_bytes: chunk_bytes,
                                    },
                                );
                                sleep_proportional(chunk_bytes);
                            }
                        }

                        for sender in &work.output_senders {
                            let copy_id = Uuid::now_v7();
                            batch_obs.init(
                                copy_id,
                                data_batch::Init {
                                    operator_id: operator.id,
                                },
                            );

                            // Place on GPU or host, then spill if needed.
                            let mut copy_gpu_index = last_gpu_index;
                            if let Some(gi) = copy_gpu_index {
                                let gpu = &self.gpus[gi];
                                gpu.memory_used.fetch_add(
                                    chunk_bytes,
                                    Ordering::Relaxed,
                                );
                                batch_obs.in_gpu_memory(
                                    copy_id,
                                    data_batch::InGpuMemory {
                                        use_gpu_memory: gpu.memory,
                                        use_gpu_memory_bytes: chunk_bytes,
                                    },
                                );
                                let gpu_pressure = gpu
                                    .memory_used
                                    .load(Ordering::Relaxed)
                                    as f64
                                    / GPU_MEMORY_CAPACITY as f64;
                                if gpu_pressure > GPU_MEMORY_SPILL_THRESHOLD {
                                    saturating_sub(&gpu.memory_used, chunk_bytes);
                                    self.host_memory_used.fetch_add(
                                        chunk_bytes,
                                        Ordering::Relaxed,
                                    );
                                    batch_obs.spilling_to_host_memory(
                                        copy_id,
                                        data_batch::SpillingToHostMemory {
                                            use_gpu_to_host_mem: gpu
                                                .gpu_to_host_mem,
                                            use_gpu_to_host_mem_bytes:
                                                chunk_bytes,
                                        },
                                    );
                                    sleep_proportional(chunk_bytes);
                                    batch_obs.in_host_memory(
                                        copy_id,
                                        data_batch::InHostMemory {
                                            use_host_memory: self
                                                .host_memory,
                                            use_host_memory_bytes:
                                                chunk_bytes,
                                        },
                                    );
                                    copy_gpu_index = None;
                                }
                            } else {
                                self.host_memory_used.fetch_add(
                                    chunk_bytes,
                                    Ordering::Relaxed,
                                );
                                batch_obs.in_host_memory(
                                    copy_id,
                                    data_batch::InHostMemory {
                                        use_host_memory: self.host_memory,
                                        use_host_memory_bytes: chunk_bytes,
                                    },
                                );
                            }

                            // Spill host→storage if over threshold.
                            let mut copy_in_storage = false;
                            if copy_gpu_index.is_none() {
                                let hp = self
                                    .host_memory_used
                                    .load(Ordering::Relaxed)
                                    as f64
                                    / HOST_MEMORY_CAPACITY as f64;
                                if hp > HOST_MEMORY_SPILL_THRESHOLD {
                                    saturating_sub(&self.host_memory_used, chunk_bytes);
                                    batch_obs.spilling_to_storage(
                                        copy_id,
                                        data_batch::SpillingToStorage {
                                            use_host_to_storage: self
                                                .host_to_storage,
                                            use_host_to_storage_bytes:
                                                chunk_bytes,
                                        },
                                    );
                                    sleep_proportional(chunk_bytes);
                                    batch_obs.in_storage(
                                        copy_id,
                                        data_batch::InStorage {
                                            use_storage: self.storage,
                                            use_storage_bytes: chunk_bytes,
                                        },
                                    );
                                    copy_in_storage = true;
                                }
                            }

                            let _ = sender.send(Batch {
                                id: copy_id,
                                bytes: chunk_bytes,
                                rows: chunk_rows,
                                gpu_index: copy_gpu_index,
                                in_storage: copy_in_storage,
                            });
                        }
                    }
                }
            }
        }

        obs.task_exit(task_id);
        operator.tasks_processed.fetch_add(1, Ordering::Relaxed);
    }

    fn execute_logical_plan(
        &self,
        context: &SimulatorContext,
        engine: &Engine,
        l_plan: &Plan<Logical>,
        num_tasks: usize,
        log_progress: bool,
    ) {
        let physical_plan = simulate_planning(l_plan);
        physical_plan.declare(context, Some(self.id));

        let nodes = petgraph::algo::toposort(&physical_plan.dag, None).unwrap();

        if physical_plan.execute {
            let plan = &physical_plan;

            // Create a channel for each DAG edge. Batches flow from source
            // operator to target operator through these channels.
            let mut edge_channels: HashMap<
                petgraph::graph::EdgeIndex,
                (Sender<Batch>, Receiver<Batch>),
            > = HashMap::new();
            for edge_idx in plan.dag.edge_indices() {
                let (tx, rx) = crossbeam_channel::unbounded();
                edge_channels.insert(edge_idx, (tx, rx));
            }

            // Build per-operator output senders and input receivers.
            let operator_outputs: HashMap<NodeIndex, Vec<&Sender<Batch>>> = nodes
                .iter()
                .map(|&node_idx| {
                    let senders = plan
                        .dag
                        .edges_directed(node_idx, Direction::Outgoing)
                        .map(|edge| &edge_channels[&edge.id()].0)
                        .collect();
                    (node_idx, senders)
                })
                .collect();

            let operator_inputs: HashMap<NodeIndex, Vec<&Receiver<Batch>>> = nodes
                .iter()
                .map(|&node_idx| {
                    let receivers = plan
                        .dag
                        .edges_directed(node_idx, Direction::Incoming)
                        .map(|edge| &edge_channels[&edge.id()].1)
                        .collect();
                    (node_idx, receivers)
                })
                .collect();

            // Work queue: scheduler sends work items, pool threads consume.
            let (work_tx, work_rx): (Sender<WorkItem>, Receiver<WorkItem>) =
                crossbeam_channel::unbounded();

            let task_counter = AtomicU64::new(0);
            let in_flight = &AtomicU64::new(0);

            // Per-operator completion counters (shared between pool threads
            // and scheduler for barrier synchronization).
            let completed: HashMap<NodeIndex, AtomicU64> = nodes
                .iter()
                .map(|&n| (n, AtomicU64::new(0)))
                .collect();
            let completed = &completed;

            // Find the output (sink) node — the root of the pull chain.
            let output_node = nodes
                .iter()
                .find(|&&n| plan.dag[n].kind == Physical::Output)
                .copied()
                .expect("physical plan must have an Output operator");

            std::thread::scope(|s| {
                // Spawn pool threads that consume work items.
                for thread_id in &self.threads {
                    let work_rx = work_rx.clone();
                    let thread_id = *thread_id;
                    s.spawn(move || {
                        while let Ok(work) = work_rx.recv() {
                            self.process_work_item(context, engine, &work, thread_id);
                            completed[&work.operator_node]
                                .fetch_add(1, Ordering::Release);
                            in_flight.fetch_sub(1, Ordering::Release);

                            // Accumulate port stats on DAG edges using
                            // actual batch values from the operator's
                            // output counters.
                            let op = &plan.dag[work.operator_node];
                            let out_bytes = op.bytes_out.load(Ordering::Relaxed);
                            let out_rows = op.rows_out.load(Ordering::Relaxed);
                            let out_batches = op.batches_out.load(Ordering::Relaxed).max(1);
                            let avg_bytes = out_bytes / out_batches;
                            let avg_rows = out_rows / out_batches;
                            let edges = plan
                                .dag
                                .edges_directed(work.operator_node, Direction::Outgoing);
                            for edge in edges {
                                edge.weight()
                                    .source
                                    .num_bytes
                                    .fetch_add(avg_bytes, Ordering::Relaxed);
                                edge.weight()
                                    .source
                                    .num_rows
                                    .fetch_add(avg_rows, Ordering::Relaxed);
                                edge.weight()
                                    .target
                                    .num_bytes
                                    .fetch_add(avg_bytes, Ordering::Relaxed);
                                edge.weight()
                                    .target
                                    .num_rows
                                    .fetch_add(avg_rows, Ordering::Relaxed);
                            }
                        }
                    });
                }

                // Pull-based scheduler: demand flows backward from Output
                // to scans; data flows forward through the DAG.
                s.spawn(|| {
                    // Per-operator demand: how many batches this operator
                    // still needs to produce for its downstream consumer(s).
                    let mut demand: HashMap<NodeIndex, usize> =
                        nodes.iter().map(|&n| (n, 0usize)).collect();

                    // Seed demand at the output node. The output wants
                    // num_tasks batches total.
                    *demand.get_mut(&output_node).unwrap() = num_tasks;

                    // Maximum batches any single scan can produce.
                    let scan_nodes: Vec<NodeIndex> = nodes
                        .iter()
                        .filter(|&&n| plan.dag[n].kind == Physical::FileSystemScan)
                        .copied()
                        .collect();
                    let max_per_scan = num_tasks / scan_nodes.len().max(1);

                    // Track how many batches each operator has been
                    // dispatched to process (for termination and demand
                    // propagation).
                    let mut dispatched: HashMap<NodeIndex, usize> =
                        nodes.iter().map(|&n| (n, 0usize)).collect();

                    // Per-barrier-operator batch buffers: collect all input
                    // batches before dispatching a single work item.
                    let mut barrier_buffers: HashMap<NodeIndex, Vec<Batch>> = nodes
                        .iter()
                        .filter(|&&n| is_barrier(plan.dag[n].kind))
                        .map(|&n| (n, Vec::new()))
                        .collect();

                    // Process in reverse topological order (output first,
                    // scans last) so demand propagates backward.
                    let reverse_topo: Vec<NodeIndex> = nodes.iter().copied().rev().collect();

                    // Total batches dispatched to Output for termination.
                    let mut output_dispatched: usize = 0;

                    loop {
                        let mut made_progress = false;

                        // Check Limit early-termination before processing
                        // any operators, to prevent demand re-propagation.
                        let limit_done = nodes.iter().any(|&n| {
                            plan.dag[n].kind == Physical::Limit
                                && plan.dag[n].rows_out.load(Ordering::Relaxed) >= 42
                        });
                        if limit_done {
                            for d in demand.values_mut() {
                                *d = 0;
                            }
                        }

                        // Forward pass (topological order): compute which
                        // nodes are effectively done — all dispatched work
                        // completed and no more input will ever arrive.
                        let mut effectively_done: HashMap<NodeIndex, bool> =
                            HashMap::new();
                        for &node_idx in &nodes {
                            let comp =
                                completed[&node_idx].load(Ordering::Acquire)
                                    as usize;
                            let disp = dispatched[&node_idx];
                            let no_inflight = comp == disp;
                            let has_dispatched = disp > 0;
                            let no_demand = demand[&node_idx] == 0;
                            let upstream_done = plan
                                .dag
                                .edges_directed(node_idx, Direction::Incoming)
                                .all(|e| {
                                    *effectively_done
                                        .get(&e.source())
                                        .unwrap_or(&false)
                                });
                            // Done if: dispatched at least once, all
                            // dispatched work completed, and either no
                            // demand left or all upstream is done (no
                            // more input will arrive).
                            let done = has_dispatched
                                && no_inflight
                                && (no_demand || upstream_done);
                            effectively_done.insert(node_idx, done);
                        }

                        for &node_idx in &reverse_topo {
                            let node_demand = demand[&node_idx];
                            if node_demand == 0 {
                                continue;
                            }

                            let op = &plan.dag[node_idx];
                            let outputs = &operator_outputs[&node_idx];

                            if op.kind == Physical::FileSystemScan {
                                // Don't over-dispatch scans.
                                if dispatched[&node_idx] >= max_per_scan {
                                    *demand.get_mut(&node_idx).unwrap() = 0;
                                    continue;
                                }
                                let idx = task_counter.fetch_add(1, Ordering::Relaxed);
                                in_flight.fetch_add(1, Ordering::Acquire);
                                let _ = work_tx.send(WorkItem {
                                    operator_node: node_idx,
                                    operator: op,
                                    input_batches: vec![],
                                    output_senders: outputs.clone(),
                                    task_index: idx,
                                });
                                *demand.get_mut(&node_idx).unwrap() -= 1;
                                *dispatched.get_mut(&node_idx).unwrap() += 1;
                                made_progress = true;
                            } else if is_barrier(op.kind) {
                                // Barrier operator: collect batches into
                                // buffer and dispatch only when all upstream
                                // operators have completed.
                                let inputs = &operator_inputs[&node_idx];
                                for rx in inputs {
                                    while let Ok(batch) = rx.try_recv() {
                                        barrier_buffers
                                            .get_mut(&node_idx)
                                            .unwrap()
                                            .push(batch);
                                        made_progress = true;
                                    }
                                }

                                // Check if all upstream operators are done
                                // using the forward-pass effectively_done map.
                                let incoming: Vec<NodeIndex> = plan
                                    .dag
                                    .edges_directed(node_idx, Direction::Incoming)
                                    .map(|e| e.source())
                                    .collect();
                                let upstream_done = incoming.iter().all(|&src| {
                                    *effectively_done.get(&src).unwrap_or(&false)
                                });

                                if upstream_done {
                                    let buffer = barrier_buffers
                                        .get_mut(&node_idx)
                                        .unwrap();
                                    if !buffer.is_empty() {
                                        let batches =
                                            std::mem::take(buffer);
                                        let idx = task_counter
                                            .fetch_add(1, Ordering::Relaxed);
                                        in_flight
                                            .fetch_add(1, Ordering::Acquire);
                                        let _ = work_tx.send(WorkItem {
                                            operator_node: node_idx,
                                            operator: op,
                                            input_batches: batches,
                                            output_senders: outputs.clone(),
                                            task_index: idx,
                                        });
                                        *demand.get_mut(&node_idx).unwrap() =
                                            0;
                                        *dispatched
                                            .get_mut(&node_idx)
                                            .unwrap() += 1;
                                        made_progress = true;
                                    }
                                } else {
                                    // Propagate demand upstream.
                                    let num_sources = incoming.len().max(1);
                                    let needed_total =
                                        node_demand + dispatched[&node_idx];
                                    let per_source =
                                        needed_total.div_ceil(num_sources);
                                    for source in incoming {
                                        let already = dispatched[&source]
                                            + demand[&source];
                                        if already < per_source {
                                            *demand
                                                .get_mut(&source)
                                                .unwrap() +=
                                                per_source - already;
                                        }
                                    }
                                }
                            } else {
                                // Pipeline operator: dispatch one task per batch.
                                let inputs = &operator_inputs[&node_idx];
                                let mut got_batch = false;
                                for rx in inputs {
                                    if let Ok(batch) = rx.try_recv() {
                                        let idx = task_counter.fetch_add(1, Ordering::Relaxed);
                                        in_flight.fetch_add(1, Ordering::Acquire);
                                        let _ = work_tx.send(WorkItem {
                                            operator_node: node_idx,
                                            operator: op,
                                            input_batches: vec![batch],
                                            output_senders: outputs.clone(),
                                            task_index: idx,
                                        });
                                        *demand.get_mut(&node_idx).unwrap() -= 1;
                                        *dispatched.get_mut(&node_idx).unwrap() += 1;
                                        if node_idx == output_node {
                                            output_dispatched += 1;
                                        }
                                        got_batch = true;
                                        made_progress = true;
                                        break;
                                    }
                                }

                                if !got_batch {
                                    // Propagate demand upstream.
                                    let incoming: Vec<NodeIndex> = plan
                                        .dag
                                        .edges_directed(node_idx, Direction::Incoming)
                                        .map(|e| e.source())
                                        .collect();
                                    let num_sources = incoming.len().max(1);
                                    let needed_total = node_demand + dispatched[&node_idx];
                                    let per_source = needed_total.div_ceil(num_sources);
                                    for source in incoming {
                                        let already = dispatched[&source] + demand[&source];
                                        if already < per_source {
                                            *demand.get_mut(&source).unwrap() +=
                                                per_source - already;
                                        }
                                    }
                                }
                            }
                        }

                        // Terminate when all demand is satisfied AND all
                        // dispatched work has completed.
                        let all_demand_zero = demand.values().all(|&d| d == 0);
                        let current_in_flight = in_flight.load(Ordering::Acquire);
                        let all_done = output_dispatched >= num_tasks || all_demand_zero;
                        if all_done && current_in_flight == 0 {
                            // Drain any remaining batches from channels that
                            // were produced by in-flight work after Limit
                            // terminated.
                            for node_idx in &reverse_topo {
                                let inputs = &operator_inputs[node_idx];
                                for rx in inputs {
                                    while let Ok(_batch) = rx.try_recv() {}
                                }
                            }
                            break;
                        }

                        if !made_progress {
                            std::thread::sleep(Duration::from_micros(10));
                        }
                    }

                    // Drop the work sender to signal pool threads to exit.
                    drop(work_tx);
                });
            });
        }

        let op_obs = context.operator_observer();
        let port_obs = context.port_observer();
        let num_nodes = nodes.len();
        for (op_index, node_idx) in nodes.iter().enumerate() {
            let op = &physical_plan.dag[*node_idx];
            let tasks_processed = op.tasks_processed.load(Ordering::Relaxed);

            let batches_in = op.batches_in.load(Ordering::Relaxed);
            let bytes_in = op.bytes_in.load(Ordering::Relaxed);
            let rows_in = op.rows_in.load(Ordering::Relaxed);
            let batches_out = op.batches_out.load(Ordering::Relaxed);
            let bytes_out = op.bytes_out.load(Ordering::Relaxed);
            let rows_out = op.rows_out.load(Ordering::Relaxed);

            // Estimate peak memory from batch throughput and operator type.
            let mem_mult: u64 = match op.kind {
                Physical::JoinLocal => 5,
                Physical::Aggregate => 4,
                Physical::GpuDecode => 3,
                Physical::Sort => 3,
                _ => 2,
            };
            let peak_memory = (bytes_in / batches_in.max(1)) * mem_mult;

            let mut attributes = vec![
                Attribute::u64("tasks_processed", tasks_processed),
                Attribute::u64("peak_memory_bytes", peak_memory),
                Attribute::u64("output_rows", rows_out),
                Attribute::u64("output_bytes", bytes_out),
                Attribute::u64("output_batches", batches_out),
                Attribute::u64("input_rows", rows_in),
                Attribute::u64("input_bytes", bytes_in),
                Attribute::u64("input_batches", batches_in),
            ];

            match op.kind {
                Physical::FileSystemScan => {
                    let selectivity: f64 = rng().random_range(0.001..1.0);
                    let bytes_read = (bytes_out as f64 / selectivity) as u64;
                    attributes.extend([
                        Attribute::u64("files_scanned", batches_out.max(1)),
                        Attribute::u64("bytes_read", bytes_read),
                        Attribute::f64("predicate_selectivity", selectivity),
                    ]);
                }
                Physical::GpuDecode => {
                    let compression_ratio: f64 = rng().random_range(2.0..8.0);
                    attributes.extend([
                        Attribute::u64("compressed_bytes", bytes_in),
                        Attribute::u64(
                            "decompressed_bytes",
                            (bytes_in as f64 * compression_ratio) as u64,
                        ),
                        Attribute::f64("compression_ratio", compression_ratio),
                    ]);
                }
                Physical::JoinPartition => {
                    let build_bytes = bytes_in / 2;
                    let probe_bytes = bytes_in - build_bytes;
                    attributes.extend([
                        Attribute::u64("build_side_bytes", build_bytes),
                        Attribute::u64("probe_side_bytes", probe_bytes),
                        Attribute::u64("network_bytes_sent", bytes_in),
                    ]);
                }
                Physical::JoinLocal => {
                    let build_rows = rows_in / 2;
                    let probe_rows = rows_in - build_rows;
                    attributes.extend([
                        Attribute::u64("hash_table_size_bytes", bytes_in / 2),
                        Attribute::u64("hash_table_entries", build_rows),
                        Attribute::u64("build_rows", build_rows),
                        Attribute::u64("probe_rows", probe_rows),
                        Attribute::u64("match_rows", rows_out),
                    ]);
                }
                Physical::Aggregate => {
                    let reduction = if rows_in > 0 {
                        rows_in as f64 / rows_out.max(1) as f64
                    } else {
                        1.0
                    };
                    attributes.extend([
                        Attribute::u64("groups_created", rows_out),
                        Attribute::f64("reduction_factor", reduction),
                    ]);
                }
                Physical::Filter => {
                    let selectivity = if rows_in > 0 {
                        rows_out as f64 / rows_in as f64
                    } else {
                        1.0
                    };
                    attributes.extend([
                        Attribute::f64("selectivity", selectivity),
                        Attribute::u64("rows_passed", rows_out),
                        Attribute::u64("rows_filtered", rows_in.saturating_sub(rows_out)),
                    ]);
                }
                Physical::Udf => {
                    attributes.extend([
                        Attribute::string("udf_name", "apply_transform"),
                        Attribute::string("udf_language", "python"),
                        Attribute::u64("rows_processed", rows_in),
                    ]);
                }
                Physical::Sort => {
                    let n = rows_in.max(1);
                    let log_n = (n as f64).log2().max(1.0) as u64;
                    attributes.extend([
                        Attribute::u64("comparison_count", n * log_n),
                        Attribute::u64("run_count", batches_in.max(1)),
                    ]);
                }
                Physical::Limit => {
                    let ratio = if rows_in > 0 {
                        1.0 - (rows_out as f64 / rows_in as f64)
                    } else {
                        0.0
                    };
                    attributes.extend([
                        Attribute::u64("amount", 42),
                        Attribute::u64("rows_inspected", rows_in),
                        Attribute::u64("rows_emitted", rows_out),
                        Attribute::f64("early_termination_ratio", ratio),
                    ]);
                }
                Physical::Output => {
                    attributes.extend([
                        Attribute::u64("rows_written", rows_in),
                        Attribute::u64("bytes_written", bytes_in),
                        Attribute::u64("flush_count", batches_in.max(1)),
                    ]);
                }
            }
            op_obs.statistics(
                op.id,
                operator::Statistics {
                    custom_attributes: attributes,
                },
            );

            let edges = physical_plan
                .dag
                .edges_directed(*node_idx, Direction::Incoming);
            for edge in edges {
                let port = &edge.weight().target;
                port_obs.statistics(
                    port.id,
                    port::Statistics {
                        custom_attributes: vec![
                            Attribute::u64("bytes", port.num_bytes.load(Ordering::Relaxed)),
                            Attribute::u64("rows", port.num_rows.load(Ordering::Relaxed)),
                        ],
                    },
                );
            }

            if log_progress {
                info!("  {}/{} {:?}", op_index + 1, num_nodes, op.kind);
            }
        }
    }

    fn shut_down(&self, context: &SimulatorContext) {
        let worker_obs = context.worker_observer();
        let memory_obs = context.memory_resource_observer();
        let channel_obs = context.channel_resource_observer();
        let processor_obs = context.processor_resource_observer();

        memory_obs.finalizing(self.storage, Default::default());
        memory_obs.exit(self.storage, Default::default());
        sleep_fixed(25);
        memory_obs.finalizing(self.host_memory, Default::default());
        memory_obs.exit(self.host_memory, Default::default());
        sleep_fixed(25);
        channel_obs.finalizing(self.storage_to_host, Default::default());
        channel_obs.exit(self.storage_to_host, Default::default());
        sleep_fixed(25);
        channel_obs.finalizing(self.host_to_storage, Default::default());
        channel_obs.exit(self.host_to_storage, Default::default());
        sleep_fixed(25);
        for thread in self.threads.iter() {
            processor_obs.finalizing(*thread, Default::default());
            processor_obs.exit(*thread, Default::default());
        }
        sleep_fixed(25);
        for gpu in self.gpus.iter() {
            memory_obs.finalizing(gpu.memory, Default::default());
            memory_obs.exit(gpu.memory, Default::default());
            processor_obs.finalizing(gpu.compute, Default::default());
            processor_obs.exit(gpu.compute, Default::default());
        }
        for gpu in &self.gpus {
            channel_obs.finalizing(gpu.host_mem_to_gpu, Default::default());
            channel_obs.exit(gpu.host_mem_to_gpu, Default::default());
            channel_obs.finalizing(gpu.gpu_to_host_mem, Default::default());
            channel_obs.exit(gpu.gpu_to_host_mem, Default::default());
        }
        sleep_fixed(25);
        worker_obs.exit(self.id);
    }
}

#[derive(Debug)]
struct Engine {
    id: Uuid,
    workers: HashMap<Uuid, Worker>,
    network: Uuid,
    network_links: HashMap<(Uuid, Uuid), Uuid>,
}

impl Engine {
    fn new() -> Self {
        Self {
            id: Uuid::now_v7(),
            workers: Default::default(),
            network: Uuid::now_v7(),
            network_links: Default::default(),
        }
    }

    fn spawn(
        &mut self,
        context: &SimulatorContext,
        num_workers: usize,
        num_threads: usize,
        num_gpus: usize,
    ) {
        // Create some observers
        info!("Simulating Engine {}", self.id);
        let engine_obs = context.engine_observer();
        let resource_group_obs = context.resource_group_observer();
        let channel_obs = context.channel_resource_observer();

        engine_obs.init(
            self.id,
            engine::Init {
                instance_name: Some(format!("holodeck-{:04x}", rng().random::<u32>())),
                implementation: Some(EngineImplementationAttributes {
                    name: Some("Simulator".into()),
                    version: Some("0.0.0-PoC".into()),
                    custom_attributes: vec![],
                }),
            },
        );

        // Workers
        let worker_ids = std::iter::repeat_with(Uuid::now_v7)
            .take(num_workers)
            .collect::<Vec<_>>();

        for (worker_index, worker_id) in worker_ids.iter().enumerate() {
            let worker = Worker::new(
                *worker_id,
                format!("drone-{worker_index}"),
                num_threads,
                num_gpus,
            );
            worker.spawn(context, self.id);
            self.workers.insert(*worker_id, worker);
        }

        // Engine-wide resources
        // Create a fully connected bidirectional network of workers
        resource_group_obs.group(
            self.network,
            resource::GroupEvent {
                type_name: "network".to_string(),
                instance_name: "network".to_string(),
                parent_group_id: Some(self.id),
            },
        );
        for worker_index in 0..worker_ids.len() {
            for other_worker_index in worker_index + 1..worker_ids.len() {
                let worker_id = worker_ids[worker_index];
                let other_worker_id = worker_ids[other_worker_index];
                let up_link_id = Uuid::now_v7();
                channel_obs.init(
                    up_link_id,
                    channel::Init {
                        resource: resource::Resource {
                            type_name: "link".to_string(),
                            instance_name: format!("worker {worker_index} -> {other_worker_index}"),
                            parent_group_id: self.network,
                        },
                        source_id: self.workers.get(&worker_id).unwrap().host_memory,
                        target_id: self.workers.get(&other_worker_id).unwrap().host_memory,
                    },
                );
                channel_obs.operating(up_link_id, channel::Operating {});

                let down_link_id = Uuid::now_v7();
                channel_obs.init(
                    down_link_id,
                    channel::Init {
                        resource: resource::Resource {
                            type_name: "link".to_string(),
                            instance_name: format!("worker {other_worker_index} -> {worker_index}"),
                            parent_group_id: self.network,
                        },
                        source_id: self.workers.get(&other_worker_id).unwrap().host_memory,
                        target_id: self.workers.get(&worker_id).unwrap().host_memory,
                    },
                );
                channel_obs.operating(down_link_id, channel::Operating {});

                self.network_links
                    .insert((worker_id, other_worker_id), up_link_id);
                self.network_links
                    .insert((other_worker_id, worker_id), down_link_id);
            }
        }
    }

    fn shut_down(&self, context: &SimulatorContext) {
        // Create some observers
        let engine_obs = context.engine_observer();
        let channel_obs = context.channel_resource_observer();

        // Tear down network
        for link in self.network_links.values().cloned() {
            channel_obs.finalizing(link, Default::default());
            channel_obs.exit(link, Default::default());
        }

        // Tear down workers
        for worker in self.workers.values() {
            worker.shut_down(context);
        }

        engine_obs.exit(self.id);
        info!("Simulated engine shut down.")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    let args = Args::parse();

    info!("Simulating with: {args:?}");

    let mut engine = Engine::new();

    let exporter = match args.exporter.as_str() {
        "postcard" => Some(ExporterOptions::Postcard(PostcardExporterOptions {
            output_dir: "data".into(),
        })),
        "messagepack" => Some(ExporterOptions::Msgpack(MsgpackExporterOptions {
            output_dir: "data".into(),
        })),
        "ndjson" => Some(ExporterOptions::Ndjson(NdjsonExporterOptions {
            output_dir: "data".into(),
        })),
        "collector" => Some(ExporterOptions::Collector(CollectorExporterOptions {
            address: args.collector_address,
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
    let context = SimulatorContext::try_new(exporter, engine.id)?;

    engine.spawn(&context, args.num_workers, args.num_threads, args.num_gpus);

    for (query_group_index, query_group_id) in std::iter::repeat_with(Uuid::now_v7)
        .take(args.num_query_groups)
        .enumerate()
    {
        let query_group_obs = context.query_group_observer();
        let query_obs = context.query_observer();

        query_group_obs.group(
            query_group_id,
            query_group::QueryGroupEvent {
                engine_id: engine.id,
                instance_name: format!("TPC-H (iteration {query_group_index})"),
            },
        );

        // "Run" the specified number of queries, sequentially for now.
        for (query_index, query_id) in std::iter::repeat_with(Uuid::now_v7)
            .take(args.num_queries)
            .enumerate()
        {
            let total = args.num_query_groups * args.num_queries;
            let done = query_group_index * args.num_queries + query_index;
            info!("{}% ({}/{})", done * 100 / total, done, total);
            query_obs.init(
                query_id,
                query::Init {
                    query_group_id,
                    instance_name: format!("Q{query_index}"),
                },
            );
            query_obs.planning(query_id);
            let l_plan = make_logical_plan(query_id, "logical".into());
            l_plan.declare(&context, None);
            query_obs.executing(query_id);

            let workers: Vec<_> = engine.workers.values().collect();
            std::thread::scope(|s| {
                let context = &context;
                let engine = &engine;
                let l_plan = &l_plan;
                for (i, worker) in workers.iter().enumerate() {
                    let log_progress = i == 0;
                    s.spawn(move || {
                        worker.execute_logical_plan(
                            context,
                            engine,
                            l_plan,
                            args.num_tasks,
                            log_progress,
                        );
                    });
                }
            });

            query_obs.exit(query_id);
        }
    }

    engine.shut_down(&context);

    drop(context);

    info!("instrumentation context dropped");
    info!("simulation completed");
    Ok(())
}
