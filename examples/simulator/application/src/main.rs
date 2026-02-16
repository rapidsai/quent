use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use clap::Parser;
use petgraph::{Directed, Direction, Graph, graph::NodeIndex, visit::EdgeRef};
use quent_attributes::Attribute;
use quent_events::resource::{self, channel, memory};
use quent_exporter_collector::CollectorExporterOptions;
use quent_instrumentation::ExporterOptions;
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

    /// Collector address (only used when --exporter is collector)
    #[arg(long)]
    collector_address: Option<String>,
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
    main_memory: Uuid,
    filesystem: Uuid,
    fs_to_mem: Uuid,
    mem_to_fs: Uuid,
    task_thread_pool: Uuid,
    task_threads: Vec<Uuid>,
}

impl Worker {
    fn new(id: Uuid, name: String, num_threads: usize) -> Self {
        Self {
            id,
            name,
            main_memory: Uuid::now_v7(),
            filesystem: Uuid::now_v7(),
            fs_to_mem: Uuid::now_v7(),
            mem_to_fs: Uuid::now_v7(),
            task_thread_pool: Uuid::now_v7(),
            task_threads: std::iter::repeat_with(Uuid::now_v7)
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

        // Main memory pool
        memory_obs.init(
            self.main_memory,
            memory::Init {
                resource: resource::Resource {
                    type_name: "memory".to_string(),
                    instance_name: "Memory".to_string(),
                    parent_group_id: self.id,
                },
            },
        );
        memory_obs.operating(self.main_memory, Default::default());

        // Filesystem -> Main memory channel
        channel_obs.init(
            self.fs_to_mem,
            channel::Init {
                resource: resource::Resource {
                    type_name: "fs2mem".to_string(),
                    instance_name: "Filesystem -> Memory".to_string(),
                    parent_group_id: self.id,
                },
                source_id: self.filesystem,
                target_id: self.main_memory,
            },
        );
        channel_obs.operating(self.fs_to_mem, Default::default());

        // Main memory -> Filesystem channel
        channel_obs.init(
            self.mem_to_fs,
            channel::Init {
                resource: resource::Resource {
                    type_name: "mem2fs".to_string(),
                    instance_name: "Memory -> Filesystem".to_string(),
                    parent_group_id: self.id,
                },
                source_id: self.main_memory,
                target_id: self.filesystem,
            },
        );
        channel_obs.operating(self.mem_to_fs, Default::default());

        // Thread pool
        resource_group_obs.group(
            self.task_thread_pool,
            resource::GroupEvent {
                type_name: "threadpool".to_string(),
                instance_name: "Thread Pool".to_string(),
                parent_group_id: Some(self.id),
            },
        );
        for (index, thread_id) in self.task_threads.iter().enumerate() {
            processor_obs.init(
                *thread_id,
                resource::processor::Init {
                    resource: resource::Resource {
                        type_name: "thread".to_string(),
                        instance_name: format!("Thread {index}"),
                        parent_group_id: self.task_thread_pool,
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
        obs.task_initializing(
            id,
            task::Init {
                operator_id: operator.id,
                instance_name: format!("task-{index}"),
            },
        );
        obs.task_queueing(id);
        let (spill, load, send) = match operator.kind {
            Physical::FileSystemScan => (false, rng().random_bool(0.5), false),
            Physical::JoinPartition => (false, rng().random_bool(0.5), true),
            Physical::JoinLocal => (true, rng().random_bool(0.5), false),
            Physical::Sort => (false, rng().random_bool(0.5), false),
            Physical::Limit => (false, rng().random_bool(0.5), false),
            Physical::Output => (false, rng().random_bool(0.5), false),
        };

        let num_bytes = rng().random_range(0..1024) * 1024 * 1024;

        obs.task_allocating_memory(
            id,
            task::AllocatingMemory {
                use_task_thread: thread,
            },
        );
        sleep_short();
        if spill {
            obs.task_allocating_storage(
                id,
                task::AllocatingStorage {
                    use_task_thread: thread,
                },
            );
            sleep_short();
            obs.task_spilling(
                id,
                task::Spilling {
                    use_task_thread: thread,
                    use_mem_to_fs: self.mem_to_fs,
                    use_mem_to_fs_bytes: num_bytes,
                },
            );
            sleep_long();
            obs.task_allocating_memory(
                id,
                task::AllocatingMemory {
                    use_task_thread: thread,
                },
            );
            sleep_short();
        }
        if load {
            obs.task_loading(
                id,
                task::Loading {
                    use_task_thread: thread,
                    use_fs_to_mem: self.fs_to_mem,
                    use_fs_to_mem_bytes: num_bytes,
                    use_main_memory: self.main_memory,
                    use_main_memory_bytes: rng().random_range(0..4) * num_bytes,
                },
            );
            sleep_long();
        }
        obs.task_computing(
            id,
            task::Computing {
                use_task_thread: thread,
                use_main_memory: self.main_memory,
                use_main_memory_bytes: rng().random_range(0..4) * num_bytes,
            },
        );
        sleep_long();
        if send {
            // Get all other workers and send some data to each of them sequentially.
            let other_workers = engine.workers.keys().filter(|w| **w != self.id);

            for other in other_workers {
                let link = *engine.network_links.get(&(self.id, *other)).unwrap();

                obs.task_sending(
                    id,
                    task::Sending {
                        use_task_thread: thread,
                        use_link: link,
                        use_link_bytes: num_bytes,
                    },
                );
                sleep_long();
            }
        }

        obs.task_finalizing(id);
        sleep_short();
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
        log_resource_links(
            engine.id,
            physical_plan.query_id,
            self.main_memory,
            "Memory",
        );
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
            self.task_thread_pool,
            "Thread Pool",
        );
        for (index, thread_id) in self.task_threads.iter().enumerate() {
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
            let tasks_per_thread_per_op = num_tasks / self.task_threads.len();
            let plan = &physical_plan;
            let nodes = &nodes;
            std::thread::scope(|s| {
                for (thread_index, thread_id) in self.task_threads.iter().enumerate() {
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
        let op_obs = context.operator_observer();
        let port_obs = context.port_observer();
        for node_idx in nodes.iter() {
            let op = &physical_plan.dag[*node_idx];
            let mut attributes = vec![Attribute::u64(
                "tasks_processed",
                op.tasks_processed.load(Ordering::Relaxed),
            )];

            match op.kind {
                Physical::FileSystemScan => {
                    attributes.push(Attribute::string("file_name", "/dev/null"))
                }
                Physical::JoinPartition => {
                    attributes.push(Attribute::u64(
                        "average_partition_size_bytes",
                        rng().random_range(1..1024 * 1024 * 1024),
                    ));
                    attributes.push(Attribute::string(
                        "join_strategy",
                        *rng().sample(Choose::new(&["broadcast", "hash partition"]).unwrap()),
                    ))
                }
                Physical::JoinLocal => (),
                Physical::Sort => attributes.push(Attribute::string(
                    "direction",
                    *rng().sample(Choose::new(&["asc", "desc"]).unwrap()),
                )),
                Physical::Limit => attributes.push(Attribute::u32("amount", 42)),
                Physical::Output => attributes.push(Attribute::string(
                    "sink",
                    *rng().sample(Choose::new(&["file", "memory"]).unwrap()),
                )),
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
        memory_obs.finalizing(self.main_memory, Default::default());
        memory_obs.exit(self.main_memory, Default::default());
        sleep_long();
        channel_obs.finalizing(self.fs_to_mem, Default::default());
        channel_obs.exit(self.fs_to_mem, Default::default());
        sleep_long();
        channel_obs.finalizing(self.mem_to_fs, Default::default());
        channel_obs.exit(self.mem_to_fs, Default::default());
        sleep_long();
        for task_thread in self.task_threads.iter() {
            processor_obs.finalizing(*task_thread, Default::default());
            processor_obs.exit(*task_thread, Default::default());
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
                instance_name: "Network".to_string(),
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
                            type_name: "Link".to_string(),
                            instance_name: format!("Worker {worker_index} -> {other_worker_index}"),
                            parent_group_id: self.network,
                        },
                        source_id: self.workers.get(&worker_id).unwrap().main_memory,
                        target_id: self.workers.get(&other_worker_id).unwrap().main_memory,
                    },
                );
                channel_obs.operating(up_link_id, channel::Operating {});

                let down_link_id = Uuid::now_v7();
                channel_obs.init(
                    down_link_id,
                    channel::Init {
                        resource: resource::Resource {
                            type_name: "Link".to_string(),
                            instance_name: format!("Worker {other_worker_index} -> {worker_index}"),
                            parent_group_id: self.network,
                        },
                        source_id: self.workers.get(&other_worker_id).unwrap().main_memory,
                        target_id: self.workers.get(&worker_id).unwrap().main_memory,
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
        "postcard" => ExporterOptions::Postcard,
        "messagepack" => ExporterOptions::Msgpack,
        "ndjson" => ExporterOptions::Ndjson,
        "collector" => ExporterOptions::Collector(CollectorExporterOptions {
            address: args.collector_address,
        }),
        _ => {
            return Err(format!(
                "invalid exporter '{}': must be postcard, messagepack, ndjson, or collector",
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

    info!("instrumentation context dropped. exiting...");

    Ok(())
}
