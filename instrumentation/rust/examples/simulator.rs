use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::Arc,
    time::Duration,
};

use clap::Parser;
use petgraph::{Directed, Direction, Graph, graph::NodeIndex, visit::EdgeRef};
use quent::ExporterOptions;
use quent_events::{
    engine::{self, EngineImplementationAttributes},
    operator, plan, q, query, query_group,
    resource::{self, channel, memory},
    worker,
};
use rand::{Rng, rng};
use rayon::prelude::*;
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
    #[arg(long, default_value_t = 2)]
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
}

fn initialize_tracing() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();
}

struct Operator<T: Debug> {
    id: Uuid,
    parents: Vec<Uuid>,
    kind: T,
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
    name: String,
    query_id: Uuid,
    // parent_plan_id: Option<Uuid>,
    dag: Graph<Operator<T>, Edge, Directed>,
    execute: bool,
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
        name,
        query_id,
        // parent_plan_id: None,
        dag,
        execute: false,
    }
}

fn make_physical_plan(logical: &Plan<Logical>) -> Plan<Physical> {
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
        name: "physical".into(),
        query_id: logical.query_id,
        // parent_plan_id: Some(logical.id),
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

    fn spawn(&self, context: &quent::Context, engine_id: Uuid) {
        let worker_obs = context.worker_observer();
        let resource_group_obs = context.resource_group_observer();
        let memory_obs = context.memory_resource_observer();
        let channel_obs = context.channel_resource_observer();
        let processor_obs = context.processor_resource_observer();

        info!("Spawning worker {}", self.name);
        let resource_links = |resource_name: &str, resource_id: Uuid| {
            info!("\tResource {resource_name}");
            info!(
                "\t\tTimeline (all FSMs): http://localhost:8080/analyzer/engine/{engine_id}/timeline/resource/{resource_id}/agg/all?num_bins=16"
            );
            info!(
                "\t\tTimeline (task FSM): http://localhost:8080/analyzer/engine/{engine_id}/timeline/resource/{resource_id}/agg/fsm?num_bins=16&fsm_type_name=task"
            );
        };

        worker_obs.init(
            self.id,
            worker::Init {
                engine_id,
                name: Some(self.name.clone()),
            },
        );

        // Filesystem
        memory_obs.init(
            self.filesystem,
            memory::Init {
                resource: resource::Resource {
                    type_name: "Filesystem".to_string(),
                    instance_name: "Filesystem".to_string(),
                    scope: resource::Scope::Worker(self.id),
                },
            },
        );
        memory_obs.operating(self.filesystem, Default::default());
        resource_links("filesystem", self.filesystem);

        // Main memory pool
        memory_obs.init(
            self.main_memory,
            memory::Init {
                resource: resource::Resource {
                    type_name: "Main Memory".to_string(),
                    instance_name: "Main Memory".to_string(),
                    scope: resource::Scope::Worker(self.id),
                },
            },
        );
        memory_obs.operating(self.main_memory, Default::default());
        resource_links("main memory", self.main_memory);

        // Filesystem -> Main memory channel
        channel_obs.init(
            self.fs_to_mem,
            channel::Init {
                resource: resource::Resource {
                    type_name: "FsToMem".to_string(),
                    instance_name: "FsToMem".to_string(),
                    scope: resource::Scope::Worker(self.id),
                },
                source_id: self.filesystem,
                target_id: self.main_memory,
            },
        );
        channel_obs.operating(self.fs_to_mem, Default::default());
        resource_links("fs to mem", self.fs_to_mem);

        // Main memory -> Filesystem channel
        channel_obs.init(
            self.mem_to_fs,
            channel::Init {
                resource: resource::Resource {
                    type_name: "MemToFs".to_string(),
                    instance_name: "MemToFs".to_string(),
                    scope: resource::Scope::Worker(self.id),
                },
                source_id: self.main_memory,
                target_id: self.filesystem,
            },
        );
        channel_obs.operating(self.mem_to_fs, Default::default());
        resource_links("mem to fs", self.mem_to_fs);

        // Thread pool
        resource_group_obs.init(
            self.task_thread_pool,
            resource::group::Init {
                resource: resource::Resource {
                    type_name: "Thread Pool".to_string(),
                    instance_name: "Thread Pool".to_string(),
                    scope: resource::Scope::Worker(self.id),
                },
            },
        );
        for (index, thread_id) in self.task_threads.iter().enumerate() {
            processor_obs.init(
                *thread_id,
                resource::processor::Init {
                    resource: resource::Resource {
                        type_name: "Task Thread".to_string(),
                        instance_name: format!("TaskThread-{index}"),
                        scope: resource::Scope::ResourceGroup(self.task_thread_pool),
                    },
                },
            );
            processor_obs.operating(*thread_id, Default::default());
            resource_links("task thread", *thread_id);
        }
        resource_group_obs.operating(self.task_thread_pool, Default::default());

        worker_obs.operating(self.id, worker::Operating {});
    }

    fn execute_physical_operator_task(
        &self,
        context: &quent::Context,
        index: usize,
        engine: &Engine,
        operator: &Operator<Physical>,
        thread: Uuid,
    ) {
        use q::task;

        let obs = context.q_observer();

        let id = Uuid::now_v7();
        obs.task_initializing(
            id,
            task::Init {
                operator_id: operator.id,
                name: Some(format!("task-{index}")),
            },
        );
        obs.task_queueing(id, task::Queueing {});
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
        std::thread::sleep(Duration::from_micros(rng().random_range(1..25)));
        if spill {
            obs.task_allocating_storage(
                id,
                task::AllocatingStorage {
                    use_task_thread: thread,
                },
            );
            std::thread::sleep(Duration::from_micros(rng().random_range(1..25)));
            obs.task_spilling(
                id,
                task::Spilling {
                    use_task_thread: thread,
                    use_mem_to_fs: self.mem_to_fs,
                    use_mem_to_fs_bytes: num_bytes,
                },
            );
            std::thread::sleep(Duration::from_millis(rng().random_range(1..25)));
            obs.task_allocating_memory(
                id,
                task::AllocatingMemory {
                    use_task_thread: thread,
                },
            );
            std::thread::sleep(Duration::from_micros(rng().random_range(1..25)));
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
            std::thread::sleep(Duration::from_millis(rng().random_range(1..25)));
        }
        obs.task_computing(
            id,
            task::Computing {
                use_task_thread: thread,
                use_main_memory: self.main_memory,
                use_main_memory_bytes: rng().random_range(0..4) * num_bytes,
            },
        );
        std::thread::sleep(Duration::from_millis(rng().random_range(1..25)));
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
                std::thread::sleep(Duration::from_millis(rng().random_range(1..25)));
            }
        }

        obs.task_finalizing(id, task::Finalizing {});
        std::thread::sleep(Duration::from_micros(10));
        obs.task_exit(id, task::Exit {});
    }

    fn execute_physical_plan(
        &self,
        context: &quent::Context,
        engine: &Engine,
        plan: &Plan<Physical>,
        num_tasks: usize,
        num_threads: usize,
    ) {
        let plan_obs = context.plan_observer();
        let operator_obs = context.operator_observer();

        let plan_id = Uuid::now_v7();
        plan_obs.init(
            plan_id,
            plan::Init {
                name: plan.name.clone(),
                query_id: plan.query_id,
                parent_plan_id: None, // TODO(johanpel): plan levels
                worker_id: Some(self.id),
                edges: plan
                    .dag
                    .edge_references()
                    .map(|edge| plan::Edge {
                        source: edge.weight().source.id,
                        target: edge.weight().target.id,
                    })
                    .collect(),
            },
        );

        plan_obs.executing(plan_id, Default::default());

        // Nonsensically create all operator events.
        let nodes = petgraph::algo::toposort(&plan.dag, None).unwrap();
        info!(
            "Topological order: {:?}",
            nodes
                .iter()
                .map(|node| format!("{:?}: {:?}", node, plan.dag[*node].kind))
                .collect::<Vec<_>>()
        );

        for node_idx in nodes.iter() {
            let op = &plan.dag[*node_idx];
            operator_obs.init(
                op.id,
                operator::Init {
                    plan_id,
                    parent_operator_ids: op.parents.clone(),
                    name: Some(op.name()),
                    ports: plan
                        .dag
                        .edges_directed(*node_idx, petgraph::Direction::Incoming)
                        .map(|edge| operator::Port {
                            id: edge.weight().target.id,
                            name: edge.weight().target.name.to_string(),
                        })
                        .chain(
                            plan.dag
                                .edges_directed(*node_idx, petgraph::Direction::Outgoing)
                                .map(|edge| operator::Port {
                                    id: edge.weight().source.id,
                                    name: edge.weight().source.name.to_string(),
                                }),
                        )
                        .collect(),
                },
            );

            operator_obs.waiting_for_inputs(op.id, Default::default());
            std::thread::sleep(Duration::from_millis(10));
            operator_obs.executing(op.id, Default::default());
        }

        if plan.execute {
            // On each thread, run a bunch of tasks for each operator.
            self.task_threads.par_iter().for_each(|task_thread_id| {
                for node_idx in nodes.iter() {
                    let op = &plan.dag[*node_idx];
                    for (index, _) in (0..num_tasks / num_threads).enumerate() {
                        self.execute_physical_operator_task(
                            context,
                            index,
                            engine,
                            op,
                            *task_thread_id,
                        );
                    }
                }
            });
        }

        for node_idx in nodes.iter() {
            let op = &plan.dag[*node_idx];
            std::thread::sleep(Duration::from_millis(10));
            operator_obs.blocked(op.id, Default::default());
            std::thread::sleep(Duration::from_millis(10));
            operator_obs.executing(op.id, Default::default());
            if plan.execute {
                // todo run some tasks
            }
            operator_obs.waiting_for_inputs(op.id, Default::default());
            std::thread::sleep(Duration::from_millis(10));
            operator_obs.finalizing(op.id, Default::default());
            std::thread::sleep(Duration::from_millis(10));
            operator_obs.exit(op.id, Default::default());
        }

        plan_obs.idle(plan_id, Default::default());
        std::thread::sleep(Duration::from_millis(10));
        plan_obs.finalizing(plan_id, Default::default());
        std::thread::sleep(Duration::from_millis(10));
        plan_obs.exit(plan_id, Default::default());
    }

    fn shut_down(&self, context: &quent::Context) {
        let worker_obs = context.worker_observer();
        let resource_group_obs = context.resource_group_observer();
        let memory_obs = context.memory_resource_observer();
        let channel_obs = context.channel_resource_observer();
        let processor_obs = context.processor_resource_observer();

        memory_obs.finalizing(self.filesystem, Default::default());
        memory_obs.exit(self.filesystem, Default::default());
        std::thread::sleep(Duration::from_millis(10));
        memory_obs.finalizing(self.main_memory, Default::default());
        memory_obs.exit(self.main_memory, Default::default());
        std::thread::sleep(Duration::from_millis(10));
        channel_obs.finalizing(self.fs_to_mem, Default::default());
        channel_obs.exit(self.fs_to_mem, Default::default());
        std::thread::sleep(Duration::from_millis(10));
        channel_obs.finalizing(self.mem_to_fs, Default::default());
        channel_obs.exit(self.mem_to_fs, Default::default());
        std::thread::sleep(Duration::from_millis(10));
        resource_group_obs.finalizing(self.task_thread_pool, Default::default());
        std::thread::sleep(Duration::from_millis(10));
        for task_thread in self.task_threads.iter() {
            processor_obs.finalizing(*task_thread, Default::default());
            processor_obs.exit(*task_thread, Default::default());
        }
        resource_group_obs.exit(self.task_thread_pool, Default::default());
        std::thread::sleep(Duration::from_millis(10));
        worker_obs.finalizing(self.id, worker::Finalizing {});
        worker_obs.exit(self.id, worker::Exit {});
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

    fn spawn(&mut self, context: &quent::Context, num_workers: usize, num_threads: usize) {
        // Create some observers
        info!(
            "Simulating Engine: http://localhost:8080/analyzer/engine/{}",
            self.id
        );
        let engine_obs = context.engine_observer();
        let resource_group_obs = context.resource_group_observer();
        let channel_obs = context.channel_resource_observer();

        engine_obs.init(
            self.id,
            engine::Init {
                name: Some(format!("holodeck-{:04x}", rng().random::<u32>())),
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
        resource_group_obs.init(
            self.network,
            resource::group::Init {
                resource: resource::Resource {
                    type_name: "Network".to_string(),
                    instance_name: "Network".to_string(),
                    scope: resource::Scope::Engine(self.id),
                },
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
                            instance_name: format!(
                                "link-worker{worker_index}-to-worker{other_worker_index}"
                            ),
                            scope: resource::Scope::ResourceGroup(self.network),
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
                            instance_name: format!(
                                "link-worker{other_worker_index}-to-worker{worker_index}"
                            ),
                            scope: resource::Scope::ResourceGroup(self.network),
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
        resource_group_obs.operating(self.network, resource::group::Operating {});

        engine_obs.operating(self.id, engine::Operating {});
    }

    fn shut_down(&self, context: &quent::Context) {
        // Create some observers
        let engine_obs = context.engine_observer();
        let resource_group_obs = context.resource_group_observer();
        let channel_obs = context.channel_resource_observer();

        engine_obs.finalizing(self.id, engine::Finalizing {});

        // Tear down network
        resource_group_obs.finalizing(self.network, Default::default());
        for link in self.network_links.values().cloned() {
            channel_obs.finalizing(link, Default::default());
            channel_obs.exit(link, Default::default());
        }
        resource_group_obs.exit(self.network, Default::default());

        // Tear down workers
        for worker in self.workers.values() {
            worker.shut_down(context);
        }

        engine_obs.exit(self.id, engine::Exit {});
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    let args = Args::parse();

    let mut engine = Engine::new();

    let context =
        quent::Context::try_new(ExporterOptions::Collector(Default::default()), engine.id)?;

    engine.spawn(&context, args.num_workers, args.num_threads);
    let engine = Arc::new(engine);
    let context = Arc::new(context);

    let query_group_futures: Vec<_> = std::iter::repeat_with(Uuid::now_v7)
        .take(args.num_query_groups)
        .map(|query_group_id| {
            info!("Simulating Query Group: http://localhost:8080/analyzer/engine/{}/query_group/{query_group_id}", engine.id);
            std::thread::spawn({
                let engine = Arc::clone(&engine);
                let context = Arc::clone(&context);
                let query_group_obs = context.query_group_observer();
                let query_obs = context.query_observer();
                let num_queries = args.num_queries;
                let num_tasks = args.num_tasks;
                let num_threads = args.num_threads;
                move || {
                    query_group_obs.init(
                        query_group_id,
                        query_group::Init {
                            engine_id: engine.id,
                            name: Some(format!("query_group-{:04x}", rng().random::<u32>())),
                        },
                    );
                    query_group_obs.operating(query_group_id, query_group::Operating {});

                    let query_futures: Vec<_> = std::iter::repeat_with(Uuid::now_v7)
                        .take(num_queries)
                        .map(|query_id| {
                            std::thread::spawn({
                                let engine = Arc::clone(&engine);
                                let context = Arc::clone(&context);
                                let query_obs = query_obs.clone();
                                move || {
                                    info!("Simulating Query: http://localhost:8080/analyzer/engine/{}/query/{query_id}", engine.id);
                                    query_obs.init(
                                        query_id,
                                        query::Init {
                                            query_group_id,
                                            name: Some(format!("query-{}", rng().random::<u32>())),
                                        },
                                    );
                                    query_obs.planning(query_id, query::Planning {});
                                    let l_plan = make_logical_plan(query_id, "logical".into());
                                    let p_plan = make_physical_plan(&l_plan);
                                    query_obs.executing(query_id, query::Executing {});

                                    // TODO(johanpel): properly nest this or don't require plans to be executable as operator fsms
                                    // engine.workers.values().next().unwrap().execute_physical_plan(&context, &l_plan);
                                    engine.workers.values().collect::<Vec<_>>().par_iter().for_each(|worker| {
                                            worker.execute_physical_plan(&context, &engine, &p_plan, num_tasks, num_threads);
                                    });

                                    query_obs.idle(query_id, query::Idle {});
                                    query_obs.finalizing(query_id, query::Finalizing {});
                                    query_obs.exit(query_id, query::Exit {});
                                }
                            })
                        })
                        .collect();

                    for query_future in query_futures {
                        query_future.join().unwrap();
                    }

                    query_group_obs.finalizing(query_group_id, query_group::Finalizing {});
                    query_group_obs.exit(query_group_id, query_group::Exit {});
                }
            })
        })
        .collect();

    for query_group_future in query_group_futures {
        query_group_future.join().unwrap();
    }

    engine.shut_down(&context);

    Ok(())
}
