use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender};

use clap::Parser;
use petgraph::{Directed, Direction, Graph, graph::NodeIndex, visit::EdgeRef};
use quent_attributes::{Attribute, List, Struct};
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
use rand::{Rng, distr::slice::Choose, rng};
use tracing::{debug, info};
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
        20000
    } else {
        25
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

#[derive(Clone, Debug)]
struct Batch {
    id: Uuid,
    bytes: u64,
    rows: u64,
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
                    instance_name: format!("{}-{node_idx:?}", op.name()),
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

#[derive(Clone, Debug, Default, Hash, PartialEq)]
struct Gpu {
    id: Uuid,
    memory: Uuid,
    compute: Uuid,
    mem_to_gpu: Uuid,
    gpu_to_mem: Uuid,
}

impl Gpu {
    fn new() -> Self {
        Self {
            id: Uuid::now_v7(),
            memory: Uuid::now_v7(),
            compute: Uuid::now_v7(),
            mem_to_gpu: Uuid::now_v7(),
            gpu_to_mem: Uuid::now_v7(),
        }
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq)]
struct Worker {
    id: Uuid,
    name: String,
    memory: Uuid,
    filesystem: Uuid,
    fs_to_mem: Uuid,
    mem_to_fs: Uuid,
    thread_pool: Uuid,
    threads: Vec<Uuid>,
    gpus: Vec<Gpu>,
}

impl Worker {
    fn new(id: Uuid, name: String, num_threads: usize, num_gpus: usize) -> Self {
        Self {
            id,
            name,
            memory: Uuid::now_v7(),
            filesystem: Uuid::now_v7(),
            fs_to_mem: Uuid::now_v7(),
            mem_to_fs: Uuid::now_v7(),
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

        info!("Spawning worker {}", self.name);
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
            self.memory,
            memory::Init {
                resource: resource::Resource {
                    type_name: "memory".to_string(),
                    instance_name: "Memory".to_string(),
                    parent_group_id: self.id,
                },
            },
        );
        memory_obs.operating(self.memory, Default::default());

        // Filesystem -> Memory channel
        channel_obs.init(
            self.fs_to_mem,
            channel::Init {
                resource: resource::Resource {
                    type_name: "fs_to_mem".to_string(),
                    instance_name: "Filesystem -> Memory".to_string(),
                    parent_group_id: self.id,
                },
                source_id: self.filesystem,
                target_id: self.memory,
            },
        );
        channel_obs.operating(self.fs_to_mem, Default::default());

        // Memory -> Filesystem channel
        channel_obs.init(
            self.mem_to_fs,
            channel::Init {
                resource: resource::Resource {
                    type_name: "mem_to_fs".to_string(),
                    instance_name: "Memory -> Filesystem".to_string(),
                    parent_group_id: self.id,
                },
                source_id: self.memory,
                target_id: self.filesystem,
            },
        );
        channel_obs.operating(self.mem_to_fs, Default::default());

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

            channel_obs.init(
                gpu.mem_to_gpu,
                channel::Init {
                    resource: resource::Resource {
                        type_name: "mem_to_gpu".to_string(),
                        instance_name: format!("Memory -> GPU {index}"),
                        parent_group_id: self.id,
                    },
                    source_id: self.memory,
                    target_id: gpu.memory,
                },
            );
            channel_obs.operating(gpu.mem_to_gpu, Default::default());

            channel_obs.init(
                gpu.gpu_to_mem,
                channel::Init {
                    resource: resource::Resource {
                        type_name: "gpu_to_mem".to_string(),
                        instance_name: format!("GPU {index} -> Memory"),
                        parent_group_id: self.id,
                    },
                    source_id: gpu.memory,
                    target_id: self.memory,
                },
            );
            channel_obs.operating(gpu.gpu_to_mem, Default::default());
        }
    }

    /// Process a single work item dispatched by the scheduler.
    /// Returns output batches to send to downstream operators.
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

        // --- Scan: create a batch from disk ---
        let input_batch = if operator.kind == Physical::FileSystemScan {
            let batch_id = Uuid::now_v7();
            let batch_bytes = rng().random_range(1..256) * 1024 * 1024;
            let batch_rows = rng().random_range(1024..65536);
            batch_obs.on_disk(
                batch_id,
                data_batch::OnDisk {
                    operator_id: operator.id,
                    use_filesystem: self.filesystem,
                    use_filesystem_bytes: batch_bytes,
                },
            );
            sleep_short();
            batch_obs.loading_to_memory(
                batch_id,
                data_batch::LoadingToMemory {
                    use_fs_to_mem: self.fs_to_mem,
                    use_fs_to_mem_bytes: batch_bytes,
                },
            );
            sleep_short();
            batch_obs.in_memory(
                batch_id,
                data_batch::InMemory {
                    use_memory: self.memory,
                    use_memory_bytes: batch_bytes,
                },
            );
            Some(Batch {
                id: batch_id,
                bytes: batch_bytes,
                rows: batch_rows,
            })
        } else {
            work.input_batch.clone()
        };

        obs.task_allocating_memory(task_id, task::Allocating { use_thread: thread });
        sleep_short();

        // --- Spill batch to disk under pressure ---
        if spill {
            obs.task_spilling(
                task_id,
                task::Spilling {
                    use_thread: thread,
                    use_mem_to_fs: self.mem_to_fs,
                    use_mem_to_fs_bytes: num_bytes,
                },
            );
            if let Some(ref batch) = input_batch {
                batch_obs.spilling_to_disk(
                    batch.id,
                    data_batch::SpillingToDisk {
                        use_mem_to_fs: self.mem_to_fs,
                        use_mem_to_fs_bytes: batch.bytes,
                    },
                );
                sleep_short();
                batch_obs.on_disk(
                    batch.id,
                    data_batch::OnDisk {
                        operator_id: operator.id,
                        use_filesystem: self.filesystem,
                        use_filesystem_bytes: batch.bytes,
                    },
                );
            }
            sleep_sometimes_really_long();
            obs.task_allocating_memory(task_id, task::Allocating { use_thread: thread });
            sleep_short();
            if let Some(ref batch) = input_batch {
                batch_obs.loading_to_memory(
                    batch.id,
                    data_batch::LoadingToMemory {
                        use_fs_to_mem: self.fs_to_mem,
                        use_fs_to_mem_bytes: batch.bytes,
                    },
                );
                sleep_short();
                batch_obs.in_memory(
                    batch.id,
                    data_batch::InMemory {
                        use_memory: self.memory,
                        use_memory_bytes: batch.bytes,
                    },
                );
            }
        }

        if load {
            obs.task_loading(
                task_id,
                task::Loading {
                    use_thread: thread,
                    use_fs_to_mem: self.fs_to_mem,
                    use_fs_to_mem_bytes: num_bytes,
                    use_memory: self.memory,
                    use_memory_bytes: rng().random_range(0..4) * num_bytes,
                },
            );
            sleep_sometimes_really_long();
        }

        obs.task_computing(
            task_id,
            task::Computing {
                use_thread: thread,
                use_memory: self.memory,
                use_memory_bytes: rng().random_range(0..4) * num_bytes,
            },
        );

        // --- GPU path ---
        if operator.kind == Physical::JoinLocal && !self.gpus.is_empty() {
            let gpu = &self.gpus[rng().random_range(0..self.gpus.len())];

            if let Some(ref batch) = input_batch {
                batch_obs.loading_to_gpu(
                    batch.id,
                    data_batch::LoadingToGpu {
                        use_mem_to_gpu: gpu.mem_to_gpu,
                        use_mem_to_gpu_bytes: batch.bytes,
                    },
                );
                sleep_short();
                batch_obs.on_gpu(
                    batch.id,
                    data_batch::OnGpu {
                        use_gpu_memory: gpu.memory,
                        use_gpu_memory_bytes: batch.bytes,
                    },
                );
            }

            obs.task_gpu_computing(
                task_id,
                task::GpuComputing {
                    use_thread: thread,
                    use_gpu_compute: gpu.compute,
                },
            );
            sleep_long();

            if let Some(ref batch) = input_batch {
                batch_obs.spilling_to_memory(
                    batch.id,
                    data_batch::SpillingToMemory {
                        use_gpu_to_mem: gpu.gpu_to_mem,
                        use_gpu_to_mem_bytes: batch.bytes,
                    },
                );
                sleep_short();
                batch_obs.in_memory(
                    batch.id,
                    data_batch::InMemory {
                        use_memory: self.memory,
                        use_memory_bytes: batch.bytes,
                    },
                );
            }
        }

        // --- Network send ---
        if send {
            let other_workers = engine.workers.keys().filter(|w| **w != self.id);
            for other in other_workers {
                let link = *engine.network_links.get(&(self.id, *other)).unwrap();
                obs.task_sending(
                    task_id,
                    task::Sending {
                        use_thread: thread,
                        use_link: link,
                        use_link_bytes: num_bytes,
                    },
                );
                sleep_long();
            }
        }

        obs.task_exit(task_id);

        // --- Produce output batches and send downstream ---
        match operator.kind {
            Physical::FileSystemScan => {
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
                    batch_obs.exit(batch.id);
                }
            }
            _ => {
                if let Some(batch) = input_batch {
                    operator.batches_in.fetch_add(1, Ordering::Relaxed);
                    operator.bytes_in.fetch_add(batch.bytes, Ordering::Relaxed);
                    operator.rows_in.fetch_add(batch.rows, Ordering::Relaxed);
                    batch_obs.exit(batch.id);

                    let output_bytes = if operator.kind == Physical::JoinLocal {
                        rng().random_range(1..4) * batch.bytes
                    } else {
                        batch.bytes / rng().random_range(1..4).max(1)
                    };
                    let output_rows = if operator.kind == Physical::JoinLocal {
                        rng().random_range(1..4) * batch.rows
                    } else {
                        batch.rows / rng().random_range(1..4).max(1)
                    };

                    let output_batch_id = Uuid::now_v7();
                    operator.batches_out.fetch_add(1, Ordering::Relaxed);
                    operator.bytes_out.fetch_add(output_bytes, Ordering::Relaxed);
                    operator.rows_out.fetch_add(output_rows, Ordering::Relaxed);

                    batch_obs.in_memory(
                        output_batch_id,
                        data_batch::InMemory {
                            use_memory: self.memory,
                            use_memory_bytes: output_bytes,
                        },
                    );

                    let output = Batch {
                        id: output_batch_id,
                        bytes: output_bytes,
                        rows: output_rows,
                    };
                    for sender in &work.output_senders {
                        let _ = sender.send(output.clone());
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
    ) {
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
        for (index, gpu) in self.gpus.iter().enumerate() {
            log_resource_group_links(
                engine.id,
                physical_plan.query_id,
                gpu.id,
                format!("GPU {index}").as_str(),
            );
            log_resource_links(
                engine.id,
                physical_plan.query_id,
                gpu.memory,
                format!("GPU {index} Memory").as_str(),
            );
            log_resource_links(
                engine.id,
                physical_plan.query_id,
                gpu.compute,
                format!("GPU {index} Compute").as_str(),
            );
            log_resource_links(
                engine.id,
                physical_plan.query_id,
                gpu.mem_to_gpu,
                format!("Memory -> GPU {index}").as_str(),
            );
            log_resource_links(
                engine.id,
                physical_plan.query_id,
                gpu.gpu_to_mem,
                format!("GPU {index} -> Memory").as_str(),
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
            let plan = &physical_plan;

            // Create a channel for each DAG edge. Batches flow from source
            // operator to target operator through these channels.
            let mut edge_channels: HashMap<petgraph::graph::EdgeIndex, (Sender<Batch>, Receiver<Batch>)> =
                HashMap::new();
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

                            // Accumulate port stats on DAG edges.
                            let edges = plan
                                .dag
                                .edges_directed(work.operator_node, Direction::Outgoing);
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
                    });
                }

                // Pull-based scheduler: demand flows backward from Output
                // to scans; data flows forward through the DAG.
                s.spawn(|| {
                    // Per-operator demand: how many batches this operator
                    // still needs to produce for its downstream consumer(s).
                    let mut demand: HashMap<NodeIndex, usize> = nodes
                        .iter()
                        .map(|&n| (n, 0usize))
                        .collect();

                    // Seed demand at the output node. The output wants
                    // num_tasks batches total.
                    *demand.get_mut(&output_node).unwrap() = num_tasks;

                    // Track how many batches each operator has produced (for
                    // termination). Output "produces" by consuming.
                    let mut produced: HashMap<NodeIndex, usize> = nodes
                        .iter()
                        .map(|&n| (n, 0usize))
                        .collect();

                    // Process in reverse topological order (output first,
                    // scans last) so demand propagates backward.
                    let reverse_topo: Vec<NodeIndex> = nodes.iter().copied().rev().collect();

                    loop {
                        let mut made_progress = false;

                        for &node_idx in &reverse_topo {
                            let node_demand = demand[&node_idx];
                            if node_demand == 0 {
                                continue;
                            }

                            let op = &plan.dag[node_idx];
                            let outputs = &operator_outputs[&node_idx];

                            if op.kind == Physical::FileSystemScan {
                                // Scans satisfy demand by producing batches.
                                let idx = task_counter.fetch_add(1, Ordering::Relaxed);
                                let _ = work_tx.send(WorkItem {
                                    operator_node: node_idx,
                                    operator: op,
                                    input_batch: None,
                                    output_senders: outputs.clone(),
                                    task_index: idx,
                                });
                                *demand.get_mut(&node_idx).unwrap() -= 1;
                                *produced.get_mut(&node_idx).unwrap() += 1;
                                made_progress = true;
                            } else {
                                // Non-scan: try to consume from input channels.
                                let inputs = &operator_inputs[&node_idx];
                                let mut got_batch = false;
                                for rx in inputs {
                                    if let Ok(batch) = rx.try_recv() {
                                        let idx = task_counter.fetch_add(1, Ordering::Relaxed);
                                        let _ = work_tx.send(WorkItem {
                                            operator_node: node_idx,
                                            operator: op,
                                            input_batch: Some(batch),
                                            output_senders: outputs.clone(),
                                            task_index: idx,
                                        });
                                        *demand.get_mut(&node_idx).unwrap() -= 1;
                                        *produced.get_mut(&node_idx).unwrap() += 1;
                                        got_batch = true;
                                        made_progress = true;
                                        break; // one batch per iteration
                                    }
                                }

                                if !got_batch {
                                    // No input data available — propagate
                                    // demand upstream so sources produce.
                                    for edge in plan
                                        .dag
                                        .edges_directed(node_idx, Direction::Incoming)
                                    {
                                        let source = edge.source();
                                        let source_produced = produced[&source];
                                        let source_demand = demand[&source];
                                        // Only add demand if the source
                                        // hasn't already been asked enough.
                                        if source_produced + source_demand < node_demand + produced[&node_idx] {
                                            *demand.get_mut(&source).unwrap() += 1;
                                        }
                                    }
                                }
                            }
                        }

                        // Check if Output has produced all requested batches.
                        if produced[&output_node] >= num_tasks {
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

        // Set some stats
        macro_rules! attr {
            (u64 $name:expr, $range:expr) => { Attribute::u64($name, rng().random_range($range)) };
            (u32 $name:expr, $val:expr) => { Attribute::u32($name, $val) };
            (f64 $name:expr, $range:expr) => { Attribute::f64($name, rng().random_range($range)) };
            (str $name:expr, $val:expr) => { Attribute::string($name, $val) };
            (pick $name:expr, $($choice:expr),+) => {
                Attribute::string($name, *rng().sample(Choose::new(&[$($choice),+]).unwrap()))
            };
        }

        let op_obs = context.operator_observer();
        let port_obs = context.port_observer();
        for node_idx in nodes.iter() {
            let op = &physical_plan.dag[*node_idx];
            let tasks_processed = op.tasks_processed.load(Ordering::Relaxed);

            let batches_in = op.batches_in.load(Ordering::Relaxed);
            let bytes_in = op.bytes_in.load(Ordering::Relaxed);
            let rows_in = op.rows_in.load(Ordering::Relaxed);
            let batches_out = op.batches_out.load(Ordering::Relaxed);
            let bytes_out = op.bytes_out.load(Ordering::Relaxed);
            let rows_out = op.rows_out.load(Ordering::Relaxed);

            // Common metrics for all operators
            let mut attributes = vec![
                Attribute::u64("tasks_processed", tasks_processed),
                attr!(u64 "wall_time_ns",       100_000..5_000_000_000),
                attr!(u64 "cpu_time_ns",        50_000..4_000_000_000),
                attr!(u64 "peak_memory_bytes",  1024..512 * 1024 * 1024),
                Attribute::u64("output_rows", rows_out),
                Attribute::u64("output_bytes", bytes_out),
                Attribute::u64("output_batches", batches_out),
                Attribute::u64("input_rows", rows_in),
                Attribute::u64("input_bytes", bytes_in),
                Attribute::u64("input_batches", batches_in),
            ];

            match op.kind {
                Physical::FileSystemScan => {
                    let num_files: u64 = rng().random_range(1..256);
                    attributes.extend([
                        attr!(str "file_name",              "/dev/null"),
                        Attribute::u64("files_scanned", num_files),
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
                        Attribute::list(
                            "per_file_bytes_read",
                            List::U64(
                                (0..num_files)
                                    .map(|_| rng().random_range(1024..1024 * 1024 * 1024))
                                    .collect(),
                            ),
                        ),
                        // Column projection info
                        Attribute::list(
                            "projected_column_names",
                            List::String(
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
                        Attribute::u64("num_partitions", num_partitions),
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
                        Attribute::list(
                            "partition_row_counts",
                            List::U64(
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
                    Attribute::list(
                        "join_keys",
                        List::String(
                            vec!["id", "region_id", "ts"]
                                .into_iter()
                                .take(rng().random_range(1..4))
                                .map(|s| s.to_string())
                                .collect(),
                        ),
                    ),
                    // Per-spill detail: list of structs with bytes + time
                    Attribute::list(
                        "spill_events",
                        List::Struct(
                            (0..rng().random_range(0u64..4))
                                .map(|_| {
                                    Struct(vec![
                                        Attribute::u64(
                                            "bytes",
                                            rng().random_range(1024..1024 * 1024 * 1024),
                                        ),
                                        Attribute::u64(
                                            "time_ns",
                                            rng().random_range(10_000..500_000_000),
                                        ),
                                        Attribute::u64("rows", rng().random_range(1000..1_000_000)),
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
                        Attribute::u64("sort_keys", num_keys as u64),
                        attr!(u64  "comparison_count",       1000..500_000_000),
                        attr!(u64  "merge_passes",           1..16),
                        attr!(u64  "run_count",              1..512),
                        attr!(u64  "spill_count",            0..64),
                        attr!(u64  "spill_bytes",            0..4u64 * 1024 * 1024 * 1024),
                        attr!(u64  "merge_time_ns",          100_000..2_000_000_000),
                        attr!(f64  "avg_key_length_bytes",   4.0..256.0),
                        attr!(f64  "presorted_fraction",     0.0..1.0),
                        // Per sort-key specification
                        Attribute::list(
                            "key_specs",
                            List::Struct(
                                [
                                    "ts", "amount", "id", "score", "name", "region", "category",
                                    "status",
                                ]
                                .iter()
                                .take(num_keys)
                                .map(|col| {
                                    Struct(vec![
                                        Attribute::string("column", *col),
                                        Attribute::string(
                                            "direction",
                                            *rng().sample(Choose::new(&["asc", "desc"]).unwrap()),
                                        ),
                                        Attribute::string(
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
                        Attribute::u64("flush_count", flush_count),
                        attr!(u64  "flush_time_ns",          10_000..500_000_000),
                        attr!(f64  "compression_ratio",      0.1..0.9),
                        attr!(u64  "serialization_time_ns",  10_000..1_000_000_000),
                        // Per-flush durations
                        Attribute::list(
                            "per_flush_time_ns",
                            List::U64(
                                (0..flush_count)
                                    .map(|_| rng().random_range(1000..10_000_000))
                                    .collect(),
                            ),
                        ),
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
        }
    }

    fn shut_down(&self, context: &SimulatorContext) {
        let worker_obs = context.worker_observer();
        let memory_obs = context.memory_resource_observer();
        let channel_obs = context.channel_resource_observer();
        let processor_obs = context.processor_resource_observer();

        memory_obs.finalizing(self.filesystem, Default::default());
        memory_obs.exit(self.filesystem, Default::default());
        sleep_long();
        memory_obs.finalizing(self.memory, Default::default());
        memory_obs.exit(self.memory, Default::default());
        sleep_long();
        channel_obs.finalizing(self.fs_to_mem, Default::default());
        channel_obs.exit(self.fs_to_mem, Default::default());
        sleep_long();
        channel_obs.finalizing(self.mem_to_fs, Default::default());
        channel_obs.exit(self.mem_to_fs, Default::default());
        sleep_long();
        for thread in self.threads.iter() {
            processor_obs.finalizing(*thread, Default::default());
            processor_obs.exit(*thread, Default::default());
        }
        sleep_long();
        for gpu in self.gpus.iter() {
            memory_obs.finalizing(gpu.memory, Default::default());
            memory_obs.exit(gpu.memory, Default::default());
            processor_obs.finalizing(gpu.compute, Default::default());
            processor_obs.exit(gpu.compute, Default::default());
            channel_obs.finalizing(gpu.mem_to_gpu, Default::default());
            channel_obs.exit(gpu.mem_to_gpu, Default::default());
            channel_obs.finalizing(gpu.gpu_to_mem, Default::default());
            channel_obs.exit(gpu.gpu_to_mem, Default::default());
        }
        sleep_long();
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
        info!("Simulating Engine:");
        info!("\thttp://localhost:8080/analyzer/engine/{}", self.id);
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
            let worker =
                Worker::new(*worker_id, format!("drone-{worker_index}"), num_threads, num_gpus);
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
                        source_id: self.workers.get(&worker_id).unwrap().memory,
                        target_id: self.workers.get(&other_worker_id).unwrap().memory,
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
                        source_id: self.workers.get(&other_worker_id).unwrap().memory,
                        target_id: self.workers.get(&worker_id).unwrap().memory,
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
        info!("Simulating Query Group:");
        info!(
            "\thttp://localhost:8080/analyzer/engine/{}/query_group/{query_group_id}/list_queries",
            engine.id
        );

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
            info!("Simulating Query:");
            info!(
                "\thttp://localhost:8080/analyzer/engine/{}/query/{query_id}",
                engine.id
            );
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
                for worker in workers {
                    s.spawn(|| {
                        worker.execute_logical_plan(
                            &context, &engine, &l_plan, args.num_tasks,
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
