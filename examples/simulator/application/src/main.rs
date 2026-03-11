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
    #[arg(long, default_value_t = 128)]
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

/// Sleep proportional to the number of bytes being processed.
fn sleep_proportional(bytes: u64) {
    let mib = (bytes / (1024 * 1024)).max(1);
    let micros = 5 + mib / 4;
    std::thread::sleep(Duration::from_micros(micros));
}

/// Occasionally very slow (1% of the time), otherwise proportional.
fn sleep_sometimes_really_long(bytes: u64) {
    let mib = (bytes / (1024 * 1024)).max(1);
    std::thread::sleep(Duration::from_micros(if rng().random_ratio(1, 100) {
        5000 + mib * 10
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
    /// Input batch (None for scan operators which produce their own).
    input_batch: Option<Batch>,
    /// Senders for the operator's outgoing edges in the DAG.
    output_senders: Vec<&'a Sender<Batch>>,
    /// Task index for naming.
    task_index: u64,
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

/// Capacity of GPU memory per device in bytes (32 GiB).
const GPU_MEMORY_CAPACITY: u64 = 32 * 1024 * 1024 * 1024;
/// Spill GPU→host when GPU memory usage exceeds 80% of capacity.
const GPU_MEMORY_SPILL_THRESHOLD: f64 = 0.80;

#[derive(Debug)]
struct Gpu {
    id: Uuid,
    memory: Uuid,
    compute: Uuid,
    /// Tracks current GPU memory usage in bytes for spill decisions.
    memory_used: AtomicU64,
}

impl Gpu {
    fn new() -> Self {
        Self {
            id: Uuid::now_v7(),
            memory: Uuid::now_v7(),
            compute: Uuid::now_v7(),
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
    /// `None` means the batch is in host memory.
    gpu_index: Option<usize>,
}

/// Capacity of host memory per worker in bytes (2 GiB).
const HOST_MEMORY_CAPACITY: u64 = 2 * 1024 * 1024 * 1024;
/// Spill threshold: spill when host memory usage exceeds 75% of capacity.
const HOST_MEMORY_SPILL_THRESHOLD: f64 = 0.75;

#[derive(Debug)]
struct Worker {
    id: Uuid,
    name: String,
    host_memory: Uuid,
    /// Tracks current host memory usage in bytes for spill decisions.
    host_memory_used: AtomicU64,
    filesystem: Uuid,
    fs_to_host_mem: Uuid,
    host_mem_to_fs: Uuid,
    host_mem_to_gpu: Uuid,
    gpu_to_host_mem: Uuid,
    thread_pool: Uuid,
    threads: Vec<Uuid>,
    gpus: Vec<Gpu>,
}

impl Worker {
    fn new(id: Uuid, name: String, num_threads: usize, num_gpus: usize) -> Self {
        Self {
            id,
            name,
            host_memory: Uuid::now_v7(),
            host_memory_used: AtomicU64::new(0),
            filesystem: Uuid::now_v7(),
            fs_to_host_mem: Uuid::now_v7(),
            host_mem_to_fs: Uuid::now_v7(),
            host_mem_to_gpu: Uuid::now_v7(),
            gpu_to_host_mem: Uuid::now_v7(),
            thread_pool: Uuid::now_v7(),
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

        // Filesystem
        memory_obs.init(
            self.filesystem,
            memory::Init {
                resource: resource::Resource {
                    type_name: "filesystem".to_string(),
                    instance_name: "Filesystem".to_string(),
                    parent_group_id: self.id,
                },
            },
        );
        memory_obs.operating(self.filesystem, Default::default());

        // Memory pool
        memory_obs.init(
            self.host_memory,
            memory::Init {
                resource: resource::Resource {
                    type_name: "host_memory".to_string(),
                    instance_name: "Host Memory".to_string(),
                    parent_group_id: self.id,
                },
            },
        );
        memory_obs.operating(self.host_memory, Default::default());

        // Filesystem -> Host Memory channel
        channel_obs.init(
            self.fs_to_host_mem,
            channel::Init {
                resource: resource::Resource {
                    type_name: "fs_to_host_mem".to_string(),
                    instance_name: "Filesystem -> Host Memory".to_string(),
                    parent_group_id: self.id,
                },
                source_id: self.filesystem,
                target_id: self.host_memory,
            },
        );
        channel_obs.operating(self.fs_to_host_mem, Default::default());

        // Host Memory -> Filesystem channel
        channel_obs.init(
            self.host_mem_to_fs,
            channel::Init {
                resource: resource::Resource {
                    type_name: "host_mem_to_fs".to_string(),
                    instance_name: "Host Memory -> Filesystem".to_string(),
                    parent_group_id: self.id,
                },
                source_id: self.host_memory,
                target_id: self.filesystem,
            },
        );
        channel_obs.operating(self.host_mem_to_fs, Default::default());

        // Thread pool
        resource_group_obs.group(
            self.thread_pool,
            resource::GroupEvent {
                type_name: "thread_pool".to_string(),
                instance_name: "Thread Pool".to_string(),
                parent_group_id: Some(self.id),
            },
        );
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

        if !self.gpus.is_empty() {
            channel_obs.init(
                self.host_mem_to_gpu,
                channel::Init {
                    resource: resource::Resource {
                        type_name: "host_mem_to_gpu".to_string(),
                        instance_name: "Host Memory -> GPU".to_string(),
                        parent_group_id: self.id,
                    },
                    source_id: self.host_memory,
                    target_id: self.gpus[0].memory,
                },
            );
            channel_obs.operating(self.host_mem_to_gpu, Default::default());

            channel_obs.init(
                self.gpu_to_host_mem,
                channel::Init {
                    resource: resource::Resource {
                        type_name: "gpu_to_host_mem".to_string(),
                        instance_name: "GPU -> Host Memory".to_string(),
                        parent_group_id: self.id,
                    },
                    source_id: self.gpus[0].memory,
                    target_id: self.host_memory,
                },
            );
            channel_obs.operating(self.gpu_to_host_mem, Default::default());
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

        // FileSystemScan: create a batch from disk (heavy I/O).
        // Each scan has a different size distribution derived from its
        // operator ID to simulate data skew across input tables.
        let mut input_batch = if operator.kind == Physical::FileSystemScan {
            let batch_id = Uuid::now_v7();
            let skew = (operator.id.as_bytes()[0] % 5) as u64 + 1; // 1-5x scale
            let base_bytes = rng().random_range(1..64) * 1024 * 1024;
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
                    use_fs_to_host_mem: self.fs_to_host_mem,
                    use_fs_to_host_mem_bytes: batch_bytes,
                },
            );
            sleep_proportional(batch_bytes * 5);
            batch_obs.in_host_memory(
                batch_id,
                data_batch::InHostMemory {
                    use_host_memory: self.host_memory,
                    use_host_memory_bytes: batch_bytes,
                },
            );
            Some(Batch {
                id: batch_id,
                bytes: batch_bytes,
                rows: batch_rows,
                gpu_index: None,
            })
        } else {
            work.input_batch.clone()
        };

        // Derive task resource usage from the actual batch size.
        let batch_bytes = input_batch.as_ref().map_or(0, |b| b.bytes);
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
        let use_gpu = matches!(
            operator.kind,
            Physical::GpuDecode
                | Physical::JoinLocal
                | Physical::JoinPartition
                | Physical::Sort
                | Physical::Udf
        ) && !self.gpus.is_empty();
        let send = operator.kind == Physical::JoinPartition;
        // Track host memory usage for this task's working set.
        self.host_memory_used
            .fetch_add(working_memory_bytes, Ordering::Relaxed);

        // Spill when host memory exceeds threshold and operator supports it.
        let memory_pressure = self.host_memory_used.load(Ordering::Relaxed) as f64
            / HOST_MEMORY_CAPACITY as f64;
        let spill = memory_pressure > HOST_MEMORY_SPILL_THRESHOLD
            && matches!(
                operator.kind,
                Physical::JoinLocal | Physical::Aggregate | Physical::Sort
            );

        obs.task_allocating_memory(task_id, task::Allocating { use_thread: thread });
        sleep_fixed(2);

        // Spill batch to disk under pressure.
        if spill {
            obs.task_spilling(
                task_id,
                task::Spilling {
                    use_thread: thread,
                },
            );
            // GPU→host first if batch is on GPU.
            if let Some(ref mut batch) = input_batch
                && let Some(gi) = batch.gpu_index
            {
                let gpu = &self.gpus[gi];
                batch_obs.spilling_to_host_memory(
                    batch.id,
                    data_batch::SpillingToHostMemory {
                        use_gpu_to_host_mem: self.gpu_to_host_mem,
                        use_gpu_to_host_mem_bytes: batch.bytes,
                    },
                );
                sleep_proportional(batch.bytes);
                gpu.memory_used.fetch_sub(batch.bytes, Ordering::Relaxed);
                batch_obs.in_host_memory(
                    batch.id,
                    data_batch::InHostMemory {
                        use_host_memory: self.host_memory,
                        use_host_memory_bytes: batch.bytes,
                    },
                );
                batch.gpu_index = None;
            }
            if let Some(ref batch) = input_batch {
                batch_obs.spilling_to_storage(
                    batch.id,
                    data_batch::SpillingToStorage {
                        use_host_mem_to_fs: self.host_mem_to_fs,
                        use_host_mem_to_fs_bytes: batch.bytes,
                    },
                );
                sleep_proportional(batch.bytes);
                batch_obs.in_storage(
                    batch.id,
                    data_batch::InStorage {
                        use_filesystem: self.filesystem,
                        use_filesystem_bytes: batch.bytes,
                    },
                );
            }
            sleep_sometimes_really_long(batch_bytes);
            obs.task_allocating_memory(task_id, task::Allocating { use_thread: thread });
            sleep_fixed(2);
            // Reload spilled batch
            if let Some(ref batch) = input_batch {
                batch_obs.loading_to_host_memory(
                    batch.id,
                    data_batch::LoadingToHostMemory {
                        use_fs_to_host_mem: self.fs_to_host_mem,
                        use_fs_to_host_mem_bytes: batch.bytes,
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
            sleep_sometimes_really_long(batch_bytes);
        }

        // Pick GPU if applicable. Prefer the GPU the batch is already on.
        let gpu_index = if use_gpu {
            input_batch
                .as_ref()
                .and_then(|b| b.gpu_index)
                .unwrap_or_else(|| rng().random_range(0..self.gpus.len()))
                .into()
        } else {
            None
        };
        let gpu = gpu_index.map(|i: usize| &self.gpus[i]);

        // Move batch to GPU memory before compute (skip if already on GPU).
        if let Some(gpu) = gpu
            && let Some(ref mut batch) = input_batch
        {
            if batch.gpu_index.is_none() {
                // Batch is in host memory — transfer to GPU.
                batch_obs.loading_to_gpu_memory(
                    batch.id,
                    data_batch::LoadingToGpuMemory {
                        use_host_mem_to_gpu: self.host_mem_to_gpu,
                        use_host_mem_to_gpu_bytes: batch.bytes,
                    },
                );
                sleep_proportional(batch.bytes);
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
            // else: batch already on this GPU, no transfer needed.
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
        obs.task_computing(
            task_id,
            task::Computing {
                use_thread: thread,
                use_host_memory: self.host_memory,
                use_host_memory_bytes: working_memory_bytes,
                use_gpu_compute: gpu.map_or(Uuid::nil(), |g| g.compute),
                use_gpu_memory: gpu.map_or(Uuid::nil(), |g| g.memory),
                use_gpu_memory_bytes: gpu_working_memory_bytes,
            },
        );
        sleep_proportional(batch_bytes * (compute_multiplier + gpu_multiplier));

        // Only spill GPU→host when GPU memory exceeds threshold.
        if let Some(gpu) = gpu
            && let Some(ref mut batch) = input_batch
        {
            let gpu_pressure =
                gpu.memory_used.load(Ordering::Relaxed) as f64 / GPU_MEMORY_CAPACITY as f64;
            if gpu_pressure > GPU_MEMORY_SPILL_THRESHOLD {
                batch_obs.spilling_to_host_memory(
                    batch.id,
                    data_batch::SpillingToHostMemory {
                        use_gpu_to_host_mem: self.gpu_to_host_mem,
                        use_gpu_to_host_mem_bytes: batch.bytes,
                    },
                );
                sleep_proportional(batch.bytes);
                gpu.memory_used.fetch_sub(batch.bytes, Ordering::Relaxed);
                batch_obs.in_host_memory(
                    batch.id,
                    data_batch::InHostMemory {
                        use_host_memory: self.host_memory,
                        use_host_memory_bytes: batch.bytes,
                    },
                );
                batch.gpu_index = None;
            }
            // else: batch stays on GPU for the next operator.
        }

        // Network send — must be from host memory.
        if send {
            if let Some(ref mut batch) = input_batch
                && let Some(gi) = batch.gpu_index
            {
                let gpu = &self.gpus[gi];
                batch_obs.spilling_to_host_memory(
                    batch.id,
                    data_batch::SpillingToHostMemory {
                        use_gpu_to_host_mem: self.gpu_to_host_mem,
                        use_gpu_to_host_mem_bytes: batch.bytes,
                    },
                );
                sleep_proportional(batch.bytes);
                gpu.memory_used.fetch_sub(batch.bytes, Ordering::Relaxed);
                batch_obs.in_host_memory(
                    batch.id,
                    data_batch::InHostMemory {
                        use_host_memory: self.host_memory,
                        use_host_memory_bytes: batch.bytes,
                    },
                );
                batch.gpu_index = None;
            }
            let other_workers = engine.workers.keys().filter(|w| **w != self.id);
            for other in other_workers {
                let link = *engine.network_links.get(&(self.id, *other)).unwrap();
                obs.task_sending(
                    task_id,
                    task::Sending {
                        use_thread: thread,
                        use_link: link,
                        use_link_bytes: batch_bytes,
                    },
                );
                sleep_proportional(batch_bytes);
            }
        }

        // Release host memory used by this task.
        self.host_memory_used
            .fetch_sub(working_memory_bytes, Ordering::Relaxed);

        obs.task_exit(task_id);

        // Produce output batches and send downstream.
        match operator.kind {
            Physical::FileSystemScan | Physical::GpuDecode => {
                // Scan and decode pass through the batch as-is.
                if let Some(batch) = input_batch {
                    operator.batches_out.fetch_add(1, Ordering::Relaxed);
                    operator.bytes_out.fetch_add(batch.bytes, Ordering::Relaxed);
                    operator.rows_out.fetch_add(batch.rows, Ordering::Relaxed);
                    for sender in &work.output_senders {
                        let _ = sender.send(batch.clone());
                    }
                }
            }
            Physical::Output => {
                if let Some(batch) = input_batch {
                    operator.batches_in.fetch_add(1, Ordering::Relaxed);
                    operator.bytes_in.fetch_add(batch.bytes, Ordering::Relaxed);
                    operator.rows_in.fetch_add(batch.rows, Ordering::Relaxed);
                    // Release GPU memory if batch was still GPU-resident.
                    if let Some(gi) = batch.gpu_index {
                        self.gpus[gi]
                            .memory_used
                            .fetch_sub(batch.bytes, Ordering::Relaxed);
                    }
                    batch_obs.exit(batch.id);
                }
            }
            _ => {
                if let Some(batch) = input_batch {
                    operator.batches_in.fetch_add(1, Ordering::Relaxed);
                    operator.bytes_in.fetch_add(batch.bytes, Ordering::Relaxed);
                    operator.rows_in.fetch_add(batch.rows, Ordering::Relaxed);
                    // Release GPU memory for the consumed input batch.
                    if let Some(gi) = batch.gpu_index {
                        self.gpus[gi]
                            .memory_used
                            .fetch_sub(batch.bytes, Ordering::Relaxed);
                    }
                    batch_obs.exit(batch.id);

                    // Compute output size based on operator semantics.
                    let (output_bytes, output_rows) = match operator.kind {
                        Physical::JoinLocal => {
                            // Joins can amplify data (build × probe).
                            let factor = rng().random_range(1..4);
                            (factor * batch.bytes, factor * batch.rows)
                        }
                        Physical::Aggregate => {
                            // Aggregation significantly reduces cardinality.
                            let denom = rng().random_range(5..15);
                            (batch.bytes / denom, batch.rows / denom.max(1))
                        }
                        Physical::Filter => {
                            // HAVING filter drops ~30% of rows.
                            let keep = rng().random_range(60..80);
                            (batch.bytes * keep / 100, batch.rows * keep / 100)
                        }
                        Physical::Udf => {
                            // Udf transforms data but preserves cardinality.
                            (batch.bytes, batch.rows)
                        }
                        Physical::Limit => {
                            // Limit early-terminates: stop producing
                            // output once 42 rows have been emitted.
                            let emitted_so_far = operator.rows_out.load(Ordering::Relaxed);
                            let remaining = 42u64.saturating_sub(emitted_so_far);
                            if remaining == 0 {
                                (0, 0)
                            } else {
                                let limit_rows = remaining.min(batch.rows);
                                let fraction = if batch.rows > 0 {
                                    limit_rows as f64 / batch.rows as f64
                                } else {
                                    1.0
                                };
                                ((batch.bytes as f64 * fraction) as u64, limit_rows)
                            }
                        }
                        _ => {
                            // JoinPartition, Sort: roughly preserve size.
                            let denom = rng().random_range(1..3);
                            (batch.bytes / denom, batch.rows / denom)
                        }
                    };

                    // Only produce output if there's data (Limit may
                    // produce 0 rows after early-termination).
                    if output_rows > 0 {
                        let output_batch_id = Uuid::now_v7();
                        operator.batches_out.fetch_add(1, Ordering::Relaxed);
                        operator
                            .bytes_out
                            .fetch_add(output_bytes, Ordering::Relaxed);
                        operator.rows_out.fetch_add(output_rows, Ordering::Relaxed);

                        batch_obs.init(
                            output_batch_id,
                            data_batch::Init {
                                operator_id: operator.id,
                            },
                        );

                        // Keep output on GPU if input was GPU-resident.
                        let output_gpu_index = if let Some(gi) = batch.gpu_index {
                            let gpu = &self.gpus[gi];
                            gpu.memory_used
                                .fetch_add(output_bytes, Ordering::Relaxed);
                            batch_obs.in_gpu_memory(
                                output_batch_id,
                                data_batch::InGpuMemory {
                                    use_gpu_memory: gpu.memory,
                                    use_gpu_memory_bytes: output_bytes,
                                },
                            );
                            Some(gi)
                        } else {
                            batch_obs.in_host_memory(
                                output_batch_id,
                                data_batch::InHostMemory {
                                    use_host_memory: self.host_memory,
                                    use_host_memory_bytes: output_bytes,
                                },
                            );
                            None
                        };

                        let output = Batch {
                            id: output_batch_id,
                            bytes: output_bytes,
                            rows: output_rows,
                            gpu_index: output_gpu_index,
                        };
                        for sender in &work.output_senders {
                            let _ = sender.send(output.clone());
                        }
                    }
                }
            }
        }

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
                                    input_batch: None,
                                    output_senders: outputs.clone(),
                                    task_index: idx,
                                });
                                *demand.get_mut(&node_idx).unwrap() -= 1;
                                *dispatched.get_mut(&node_idx).unwrap() += 1;
                                made_progress = true;
                            } else {
                                let inputs = &operator_inputs[&node_idx];
                                let mut got_batch = false;
                                for rx in inputs {
                                    if let Ok(batch) = rx.try_recv() {
                                        let idx = task_counter.fetch_add(1, Ordering::Relaxed);
                                        in_flight.fetch_add(1, Ordering::Acquire);
                                        let _ = work_tx.send(WorkItem {
                                            operator_node: node_idx,
                                            operator: op,
                                            input_batch: Some(batch),
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

        memory_obs.finalizing(self.filesystem, Default::default());
        memory_obs.exit(self.filesystem, Default::default());
        sleep_fixed(25);
        memory_obs.finalizing(self.host_memory, Default::default());
        memory_obs.exit(self.host_memory, Default::default());
        sleep_fixed(25);
        channel_obs.finalizing(self.fs_to_host_mem, Default::default());
        channel_obs.exit(self.fs_to_host_mem, Default::default());
        sleep_fixed(25);
        channel_obs.finalizing(self.host_mem_to_fs, Default::default());
        channel_obs.exit(self.host_mem_to_fs, Default::default());
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
        if !self.gpus.is_empty() {
            channel_obs.finalizing(self.host_mem_to_gpu, Default::default());
            channel_obs.exit(self.host_mem_to_gpu, Default::default());
            channel_obs.finalizing(self.gpu_to_host_mem, Default::default());
            channel_obs.exit(self.gpu_to_host_mem, Default::default());
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
