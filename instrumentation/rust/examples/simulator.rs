use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use petgraph::{Directed, Direction, Graph, graph::NodeIndex, visit::EdgeRef};
use quent::{ExporterOptions, OperatorObserver, PlanObserver};
use quent_events::{
    engine::{self, EngineImplementationAttributes},
    operator, plan, query, query_group,
    resource::{self, channel, memory},
    worker,
};

use rand::{Rng, rng};
use tracing::info;
use uuid::Uuid;

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

type LogicalOp = Operator<Logical>;
type LogicalPlan = Graph<LogicalOp, Edge, Directed>;

type PhysicalOp = Operator<Physical>;
type PhysicalPlan = Graph<PhysicalOp, Edge, Directed>;

// Create the following logical plan:
// Scan -> Project \
//                  -> Join -> Sort -> Limit -> Output
// Scan -> Project /
fn logical_plan() -> LogicalPlan {
    // Add a scan --> project branch and return the (project, project output port) Uuids.
    fn add_scan_project_branch(plan: &mut LogicalPlan) -> NodeIndex {
        let scan = plan.add_node(LogicalOp::new(Logical::Scan, vec![]));
        let project = plan.add_node(LogicalOp::new(Logical::Project, vec![]));
        plan.add_edge(scan, project, Edge::new("out", "in"));

        project
    }

    let mut plan = Graph::new();

    let project_a = add_scan_project_branch(&mut plan);
    let project_b = add_scan_project_branch(&mut plan);

    let join = plan.add_node(LogicalOp::new(Logical::Join, vec![]));
    plan.add_edge(project_a, join, Edge::new("out", "left"));
    plan.add_edge(project_b, join, Edge::new("out", "right"));

    let sort = plan.add_node(LogicalOp::new(Logical::Sort, vec![]));
    plan.add_edge(join, sort, Edge::new("out", "in"));

    let limit = plan.add_node(LogicalOp::new(Logical::Limit, vec![]));
    plan.add_edge(sort, limit, Edge::new("out", "in"));

    let output = plan.add_node(LogicalOp::new(Logical::Output, vec![]));
    plan.add_edge(limit, output, Edge::new("out", "in"));

    plan
}

fn make_physical_plan(logical: &LogicalPlan) -> PhysicalPlan {
    // Find the output node
    let output = logical
        .node_indices()
        .collect::<Vec<_>>()
        .into_iter()
        .find(|n| logical[*n].kind == Logical::Output)
        .unwrap();

    // Build a physical plan
    let mut physical = PhysicalPlan::new();

    lower_logical(logical, &mut physical, output, None);

    physical
}

fn lower_logical(
    logical: &LogicalPlan,
    physical: &mut PhysicalPlan,
    logical_current_idx: NodeIndex,
    physical_target_idx_port: Option<(NodeIndex, &'static str)>,
) {
    let current_logical_op = &logical[logical_current_idx];

    match current_logical_op.kind {
        Logical::Scan => {
            unimplemented!("this shouldn't happen in this simulator, yet")
        }
        Logical::Project => {
            // Check whether this project has an incoming scan source to simulate predicate pushdown
            if let Some(scan_edge) = logical
                .edges_directed(logical_current_idx, Direction::Incoming)
                .find(|edge| logical[edge.source()].kind == Logical::Scan)
            {
                let scan_op = &logical[scan_edge.source()];
                let source = physical.add_node(PhysicalOp::new(
                    Physical::FileSystemScan,
                    vec![current_logical_op.id, scan_op.id],
                ));
                if let Some((target_node, target_port)) = physical_target_idx_port {
                    physical.add_edge(source, target_node, Edge::new(target_port, "in"));
                }
            } else {
                unimplemented!("this shouldn't happen in this simulator, yet");
            }
        }
        Logical::Join => {
            // split up in a partition stage and join stage
            let partition = physical.add_node(PhysicalOp::new(
                Physical::JoinPartition,
                vec![current_logical_op.id],
            ));
            let local = physical.add_node(PhysicalOp::new(
                Physical::JoinLocal,
                vec![current_logical_op.id],
            ));
            physical.add_edge(partition, local, Edge::new("build_out", "build_in"));
            physical.add_edge(partition, local, Edge::new("probe_out", "probe_in"));

            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical.add_edge(local, target_node, Edge::new("out", target_port));
            }

            // Recurse up both branches
            for input_edge in logical.edges_directed(logical_current_idx, Direction::Incoming) {
                lower_logical(
                    logical,
                    physical,
                    input_edge.source(),
                    Some((partition, input_edge.weight().target.name)),
                );
            }
        }
        Logical::Sort => {
            let sort =
                physical.add_node(PhysicalOp::new(Physical::Sort, vec![current_logical_op.id]));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical.add_edge(sort, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
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
            let limit = physical.add_node(PhysicalOp::new(
                Physical::Limit,
                vec![current_logical_op.id],
            ));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical.add_edge(limit, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
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
            let output = physical.add_node(PhysicalOp::new(
                Physical::Output,
                vec![current_logical_op.id],
            ));
            if let Some((target_node, target_port)) = physical_target_idx_port {
                physical.add_edge(output, target_node, Edge::new("out", target_port));
            }
            let input_edge = logical
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

fn create_plan_events<T>(
    query_id: Uuid,
    plan_obs: &PlanObserver,
    op_obs: &OperatorObserver,
    plan: &Graph<Operator<T>, Edge, Directed>,
    plan_name: String,
    parent_plan_id: Option<Uuid>,
) -> Uuid
where
    T: Debug,
{
    let plan_id = Uuid::now_v7();

    plan_obs.init(
        plan_id,
        plan::Init {
            name: plan_name,
            query_id,
            parent_plan_id,
            worker_id: None,
            edges: plan
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
    let nodes = petgraph::algo::toposort(plan, None).unwrap();
    info!(
        "Topological order: {:?}",
        nodes
            .iter()
            .map(|node| format!("{:?}: {:?}", node, plan[node.clone()].kind))
            .collect::<Vec<_>>()
    );

    for node_idx in nodes.into_iter() {
        let op = &plan[node_idx];
        op_obs.init(
            op.id,
            operator::Init {
                plan_id,
                parent_operator_ids: op.parents.clone(),
                name: Some(op.name()),
                ports: plan
                    .edges_directed(node_idx, petgraph::Direction::Incoming)
                    .map(|edge| operator::Port {
                        id: edge.weight().target.id,
                        name: edge.weight().target.name.to_string(),
                    })
                    .chain(
                        plan.edges_directed(node_idx, petgraph::Direction::Outgoing)
                            .map(|edge| operator::Port {
                                id: edge.weight().source.id,
                                name: edge.weight().source.name.to_string(),
                            }),
                    )
                    .collect(),
            },
        );

        op_obs.waiting_for_inputs(op.id, Default::default());
        op_obs.executing(op.id, Default::default());
        op_obs.blocked(op.id, Default::default());
        op_obs.executing(op.id, Default::default());
        op_obs.waiting_for_inputs(op.id, Default::default());
        op_obs.finalizing(op.id, Default::default());
        op_obs.exit(op.id, Default::default());
    }

    plan_obs.idle(plan_id, Default::default());
    plan_obs.finalizing(plan_id, Default::default());
    plan_obs.exit(plan_id, Default::default());

    plan_id
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    let engine_id = Uuid::now_v7();

    info!("simulating engine - http://localhost:8080/analyzer/engine/{engine_id}");

    let context =
        quent::Context::try_new(ExporterOptions::Collector(Default::default()), engine_id)?;

    info!("context created, creating events...");

    // Spawn engine
    let engine_obs = context.engine_observer();
    engine_obs.init(
        engine_id,
        engine::Init {
            name: Some(format!("holodeck-{:04x}", rng().random::<u32>())),
            implementation: Some(EngineImplementationAttributes {
                name: Some("Simulator".into()),
                version: Some("0.0.0-PoC".into()),
                custom_attributes: vec![],
            }),
        },
    );

    // Create some observers
    let worker_obs = context.worker_observer();
    let resource_group_obs = context.resource_group_observer();
    let memory_obs = context.memory_resource_observer();
    let channel_obs = context.channel_resource_observer();
    let processor_obs = context.processor_resource_observer();

    // Engine resources.
    let network_id = Uuid::now_v7();
    resource_group_obs.init(
        network_id,
        resource::group::Init {
            resource: resource::Resource {
                name: "Network".to_string(),
                scope: resource::Scope::Worker(engine_id),
            },
        },
    );
    let mut network_links: HashMap<(Uuid, Uuid), (Uuid, Uuid)> = HashMap::new();

    // Worker resources.
    type ResourceMap = HashMap<Uuid, Uuid>;
    let mut worker_filesystem = ResourceMap::new();
    let mut worker_main_memory = ResourceMap::new();
    let mut worker_fs_to_mem = ResourceMap::new();
    let mut worker_mem_to_fs = ResourceMap::new();
    let mut worker_thread_pool = ResourceMap::new();
    let mut worker_task_thread: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    let worker_ids = std::iter::repeat_with(|| Uuid::now_v7())
        .take(4)
        .collect::<Vec<_>>();
    for (worker_index, worker_id) in worker_ids.iter().enumerate() {
        worker_obs.init(
            *worker_id,
            worker::Init {
                engine_id,
                name: Some(format!("worker-{worker_index}")),
            },
        );

        // Spawn resources in workers
        let filesystem_id = Uuid::now_v7();
        memory_obs.init(
            filesystem_id,
            memory::Init {
                resource: resource::Resource {
                    name: "Filesystem".to_string(),
                    scope: resource::Scope::Worker(*worker_id),
                },
            },
        );
        memory_obs.operating(filesystem_id, Default::default());
        worker_filesystem.insert(*worker_id, filesystem_id);

        let main_memory_id = Uuid::now_v7();
        memory_obs.init(
            main_memory_id,
            memory::Init {
                resource: resource::Resource {
                    name: "Filesystem".to_string(),
                    scope: resource::Scope::Worker(*worker_id),
                },
            },
        );
        memory_obs.operating(main_memory_id, Default::default());
        worker_main_memory.insert(*worker_id, main_memory_id);

        let fs_to_mem_id = Uuid::now_v7();
        channel_obs.init(
            fs_to_mem_id,
            channel::Init {
                resource: resource::Resource {
                    name: "FsToMem".to_string(),
                    scope: resource::Scope::Worker(*worker_id),
                },
                source_id: filesystem_id,
                target_id: main_memory_id,
            },
        );
        channel_obs.operating(fs_to_mem_id, Default::default());
        worker_fs_to_mem.insert(*worker_id, fs_to_mem_id);

        let mem_to_fs_id = Uuid::now_v7();
        channel_obs.init(
            mem_to_fs_id,
            channel::Init {
                resource: resource::Resource {
                    name: "MemToFs".to_string(),
                    scope: resource::Scope::Worker(*worker_id),
                },
                source_id: main_memory_id,
                target_id: filesystem_id,
            },
        );
        channel_obs.operating(mem_to_fs_id, Default::default());
        worker_mem_to_fs.insert(*worker_id, mem_to_fs_id);

        let thread_pool_id = Uuid::now_v7();
        resource_group_obs.init(
            thread_pool_id,
            resource::group::Init {
                resource: resource::Resource {
                    name: "ThreadPool".to_string(),
                    scope: resource::Scope::Worker(*worker_id),
                },
            },
        );
        worker_thread_pool.insert(*worker_id, thread_pool_id);
        {
            let thread_ids: Vec<_> = std::iter::repeat_with(|| Uuid::now_v7()).take(4).collect();
            for (index, thread_id) in thread_ids.iter().enumerate() {
                processor_obs.init(
                    *thread_id,
                    resource::processor::Init {
                        resource: resource::Resource {
                            name: format!("TaskThread-{index}"),
                            scope: resource::Scope::ResourceGroup(thread_pool_id),
                        },
                    },
                );
                processor_obs.operating(*thread_id, Default::default());
            }
            worker_task_thread.insert(*worker_id, thread_ids);
        }
        resource_group_obs.operating(thread_pool_id, Default::default());
    }

    // Create a fully connected bidirectional network of workers
    for worker_index in 0..worker_ids.len() {
        for other_worker_index in worker_index + 1..worker_ids.len() {
            let worker_id = worker_ids[worker_index];
            let other_worker_id = worker_ids[other_worker_index];
            let up_link_id = Uuid::now_v7();
            channel_obs.init(
                up_link_id,
                channel::Init {
                    resource: resource::Resource {
                        name: format!("link-{worker_index}-to-{other_worker_index}"),
                        scope: resource::Scope::ResourceGroup(network_id),
                    },
                    source_id: *worker_main_memory.get(&worker_id).unwrap(),
                    target_id: *worker_main_memory.get(&other_worker_id).unwrap(),
                },
            );
            let down_link_id = Uuid::now_v7();
            channel_obs.init(
                down_link_id,
                channel::Init {
                    resource: resource::Resource {
                        name: format!("link-{other_worker_index}-to-{worker_index}"),
                        scope: resource::Scope::ResourceGroup(network_id),
                    },
                    source_id: *worker_main_memory.get(&other_worker_id).unwrap(),
                    target_id: *worker_main_memory.get(&worker_id).unwrap(),
                },
            );
            network_links.insert((worker_id, other_worker_id), (up_link_id, down_link_id));
        }
    }

    for worker_id in worker_ids.iter() {
        worker_obs.operating(*worker_id, worker::Operating {});
    }

    engine_obs.operating(engine_id, engine::Operating {});

    let query_group_futures: Vec<_> = std::iter::repeat_with(|| Uuid::now_v7())
        .take(2)
        .map(|query_group_id| {
            info!("simulating query_group - http://localhost:8080/analyzer/engine/{engine_id}/query_group/{query_group_id}");
            std::thread::spawn({
                let engine_id = engine_id.clone();
                let query_group_obs = context.query_group_observer();
                let query_obs = context.query_observer();
                let plan_obs = context.plan_observer();
                let operator_obs = context.operator_observer();

                move || {
                    query_group_obs.init(
                        query_group_id,
                        query_group::Init {
                            engine_id,
                            name: Some(format!("query_group-{:04x}", rng().random::<u32>())),
                        },
                    );
                    query_group_obs.operating(query_group_id, query_group::Operating {});

                    let query_futures: Vec<_> = std::iter::repeat_with(|| Uuid::now_v7())
                        .take(2)
                        .map(|query_id| {
                            std::thread::spawn({
                                let query_obs = query_obs.clone();
                                let plan_obs = plan_obs.clone();
                                let operator_obs = operator_obs.clone();
                                move || {
                                    info!("simulating query - http://localhost:8080/analyzer/engine/{engine_id}/query/{query_id}");
                                    query_obs.init(
                                        query_id,
                                        query::Init {
                                            query_group_id,
                                            name: Some(format!("query-{}", rng().random::<u32>())),
                                        },
                                    );
                                    query_obs.planning(query_id, query::Planning {});
                                    let l_plan = logical_plan();
                                    let p_plan = make_physical_plan(&l_plan);
                                    query_obs.executing(query_id, query::Executing {});

                                    let logical_plan_id = create_plan_events(query_id, &plan_obs, &operator_obs, &l_plan, "logical".to_string(), None);
                                    // TODO(johanpel): properly nest this
                                    let _physical_plan_id = create_plan_events(query_id, &plan_obs, &operator_obs, &p_plan, "physical".to_string(), Some(logical_plan_id));

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

    engine_obs.finalizing(engine_id, engine::Finalizing {});

    // Shut down workers.
    for worker_id in worker_ids.iter() {
        let filesystem = *worker_filesystem.get(worker_id).unwrap();
        memory_obs.finalizing(filesystem, Default::default());
        memory_obs.exit(filesystem, Default::default());

        let main_memory = *worker_main_memory.get(worker_id).unwrap();
        memory_obs.finalizing(main_memory, Default::default());
        memory_obs.exit(main_memory, Default::default());

        let fs_to_mem = *worker_fs_to_mem.get(worker_id).unwrap();
        channel_obs.finalizing(fs_to_mem, Default::default());
        channel_obs.exit(fs_to_mem, Default::default());

        let mem_to_fs = *worker_mem_to_fs.get(worker_id).unwrap();
        channel_obs.finalizing(mem_to_fs, Default::default());
        channel_obs.exit(mem_to_fs, Default::default());

        for task_thread in worker_task_thread.get(worker_id).unwrap() {
            processor_obs.finalizing(*task_thread, Default::default());
            processor_obs.exit(*task_thread, Default::default());
        }

        let thread_pool = *worker_thread_pool.get(worker_id).unwrap();
        resource_group_obs.finalizing(thread_pool, Default::default());
        resource_group_obs.exit(thread_pool, Default::default());

        worker_obs.finalizing(*worker_id, worker::Finalizing {});
        worker_obs.exit(*worker_id, worker::Exit {});
    }

    for (up_link, down_link) in network_links.values().cloned() {
        channel_obs.finalizing(up_link, Default::default());
        channel_obs.exit(down_link, Default::default());
    }
    resource_group_obs.finalizing(network_id, Default::default());
    resource_group_obs.exit(network_id, Default::default());

    engine_obs.exit(engine_id, engine::Exit {});

    info!("events pushed, waiting 1s to flush (for now :tm:)");
    // TODO(johanpel): ensure the channels are flushed on drop
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}
