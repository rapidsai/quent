// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

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
use quent_simulator_events::task;
use quent_simulator_instrumentation::SimulatorContext;
use rand::{Rng, distr::slice::Choose, rng};
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
struct Worker {
    id: Uuid,
    name: String,
    memory: Uuid,
    filesystem: Uuid,
    fs_to_mem: Uuid,
    mem_to_fs: Uuid,
    thread_pool: Uuid,
    threads: Vec<Uuid>,
}

impl Worker {
    fn new(id: Uuid, name: String, num_threads: usize) -> Self {
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
    }

    fn execute_physical_operator_task(
        &self,
        context: &SimulatorContext,
        index: usize,
        engine: &Engine,
        operator: &Operator<Physical>,
        thread: Uuid,
    ) {
        let obs = context.task_observer();

        let id = Uuid::now_v7();
        obs.task_queueing(
            id,
            task::Queueing {
                operator_id: operator.id,
                instance_name: format!("task-{index}"),
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

        obs.task_allocating_memory(id, task::Allocating { use_thread: thread });
        sleep_short();
        if spill {
            obs.task_spilling(
                id,
                task::Spilling {
                    use_thread: thread,
                    use_mem_to_fs: self.mem_to_fs,
                    use_mem_to_fs_bytes: num_bytes,
                },
            );
            sleep_sometimes_really_long();
            obs.task_allocating_memory(id, task::Allocating { use_thread: thread });
            sleep_short();
        }
        if load {
            obs.task_loading(
                id,
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
            id,
            task::Computing {
                use_thread: thread,
                use_memory: self.memory,
                use_memory_bytes: rng().random_range(0..4) * num_bytes,
            },
        );

        if operator.kind == Physical::JoinLocal {
            simulate_cudf_trace(context, id)
        };

        if send {
            // Get all other workers and send some data to each of them sequentially.
            let other_workers = engine.workers.keys().filter(|w| **w != self.id);

            for other in other_workers {
                let link = *engine.network_links.get(&(self.id, *other)).unwrap();

                obs.task_sending(
                    id,
                    task::Sending {
                        use_thread: thread,
                        use_link: link,
                        use_link_bytes: num_bytes,
                    },
                );
                sleep_long();
            }
        }

        obs.task_exit(id);
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
            let plan = &physical_plan;
            let nodes = &nodes;
            std::thread::scope(|s| {
                for (thread_index, thread_id) in self.threads.iter().enumerate() {
                    s.spawn({
                        let thread_id = *thread_id;
                        move || {
                            for task_index in thread_index * tasks_per_thread_per_op
                                ..(thread_index + 1) * tasks_per_thread_per_op
                            {
                                for node_idx in nodes {
                                    let op = &plan.dag[*node_idx];
                                    self.execute_physical_operator_task(
                                        context, task_index, engine, op, thread_id,
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
                        }
                    });
                }
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

            // Common metrics for all operators
            let mut attributes = vec![
                Attribute::u64("tasks_processed", tasks_processed),
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

    fn spawn(&mut self, context: &SimulatorContext, num_workers: usize, num_threads: usize) {
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
            let worker = Worker::new(*worker_id, format!("drone-{worker_index}"), num_threads);
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

fn simulate_cudf_trace(context: &SimulatorContext, task_id: Uuid) {
    // Nothing in this simulator aims to make too much sense to begin with, so this code is LLM generated.
    let trace = context.trace_observer(task_id);

    let kernel = trace.span(task_id, "kernel".into(), None);
    kernel.enter(vec![]);

    // Device memory allocation
    let alloc = trace.span(task_id, "rmm::alloc".into(), Some(kernel.span_id()));
    alloc.enter(vec![]);
    sleep_short();
    alloc.exit(vec![]);
    alloc.close();

    // Host-to-device transfer
    let h2d = trace.span(task_id, "cudf::h2d_copy".into(), Some(kernel.span_id()));
    h2d.enter(vec![]);
    sleep_short();
    h2d.exit(vec![]);
    h2d.close();

    // Column operations
    let col_ops = trace.span(task_id, "cudf::column_ops".into(), Some(kernel.span_id()));
    col_ops.enter(vec![]);

    // Decompress input column
    let decompress = trace.span(
        task_id,
        "nvcomp::decompress".into(),
        Some(col_ops.span_id()),
    );
    decompress.enter(vec![]);
    sleep_short();
    decompress.exit(vec![]);
    decompress.close();

    // Apply filter predicate
    if rng().random_bool(0.7) {
        let filter = trace.span(
            task_id,
            "cudf::apply_filter".into(),
            Some(col_ops.span_id()),
        );
        filter.enter(vec![]);

        let pred = trace.span(
            task_id,
            "cudf::eval_predicate".into(),
            Some(filter.span_id()),
        );
        pred.enter(vec![]);
        sleep_short();
        pred.exit(vec![]);
        pred.close();

        let gather = trace.span(task_id, "cudf::gather".into(), Some(filter.span_id()));
        gather.enter(vec![]);
        sleep_short();
        gather.exit(vec![]);
        gather.close();

        filter.exit(vec![]);
        filter.close();
    }

    // Hash partitioning
    let hash = trace.span(
        task_id,
        "cudf::hash_partition".into(),
        Some(col_ops.span_id()),
    );
    hash.enter(vec![]);

    let murmur = trace.span(task_id, "cudf::murmur_hash".into(), Some(hash.span_id()));
    murmur.enter(vec![]);
    sleep_long();
    murmur.exit(vec![]);
    murmur.close();

    let scatter = trace.span(task_id, "cudf::scatter".into(), Some(hash.span_id()));
    scatter.enter(vec![]);
    sleep_short();
    scatter.exit(vec![]);
    scatter.close();

    hash.exit(vec![]);
    hash.close();

    col_ops.exit(vec![]);
    col_ops.close();

    // Device-to-host transfer
    let d2h = trace.span(task_id, "cudf::d2h_copy".into(), Some(kernel.span_id()));
    d2h.enter(vec![]);
    sleep_short();
    d2h.exit(vec![]);
    d2h.close();

    // Free device memory
    let free = trace.span(task_id, "rmm::free".into(), Some(kernel.span_id()));
    free.enter(vec![]);
    sleep_short();
    free.exit(vec![]);
    free.close();

    kernel.exit(vec![]);
    kernel.close();
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

    engine.spawn(&context, args.num_workers, args.num_threads);

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
                        worker.execute_logical_plan(&context, &engine, &l_plan, args.num_tasks);
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
